
#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

use anyhow::{Error as E, Result};
use log::{debug, info};
use std::io::Write;

use candle_transformers::models::gemma::{Config, Model};
use tokio::sync::mpsc::Sender;

use candle_core::{DType, Device, Tensor};
use candle_examples::token_output_stream::TokenOutputStream;
use candle_nn::VarBuilder;
use candle_transformers::generation::LogitsProcessor;
use hf_hub::{api::sync::Api, Repo, RepoType};
use safetensors::tensor::View;
use std::sync::Arc;
use tokenizers::Tokenizer;
use tokio::sync::Mutex;

struct TextGeneration {
    model: Model,
    device: Device,
    tokenizer: TokenOutputStream,
    logits_processor: LogitsProcessor,
    repeat_penalty: f32,
    repeat_last_n: usize,
    internal_token_sender: Sender<String>,
}

impl TextGeneration {
    #[allow(clippy::too_many_arguments)]
    fn new(
        model: Model,
        tokenizer: Tokenizer,
        seed: u64,
        temp: Option<f64>,
        top_p: Option<f64>,
        repeat_penalty: f32,
        repeat_last_n: usize,
        device: &Device,
        internal_token_sender: Sender<String>,
    ) -> Self {
        let logits_processor = LogitsProcessor::new(seed, temp, top_p);
        Self {
            model,
            tokenizer: TokenOutputStream::new(tokenizer),
            logits_processor,
            repeat_penalty,
            repeat_last_n,
            device: device.clone(),
            internal_token_sender,
        }
    }

    async fn run(&mut self, prompt: &str, sample_len: usize) -> Result<()> {
        let verbose_prompt: bool = false;
        let clear_kv_cache = true;

        if clear_kv_cache {
            self.model.clear_kv_cache();
        }
        self.tokenizer.clear();
        let mut tokens = self
            .tokenizer
            .tokenizer()
            .encode(prompt, true)
            .map_err(E::msg)?
            .get_ids()
            .to_vec();

        for &t in tokens.iter() {
            if let Some(t) = self.tokenizer.next_token(t)? {
                if verbose_prompt {
                    println!("'{}'", t);
                    std::io::stdout().flush()?;
                }
            }
        }

        // Skip the first token
        for &t in tokens.iter() {
            if let Some(_) = self.tokenizer.next_token(t)? {
                break;
            }
        }

        debug!("prompt: {:?}", prompt);

        let eos_token = match self.tokenizer.get_token("<eos>") {
            Some(token) => token,
            None => anyhow::bail!("cannot find the <eos> token"),
        };
        for index in 0..sample_len {
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let start_pos = tokens.len().saturating_sub(context_size);
            let ctxt = &tokens[start_pos..];
            let input = Tensor::new(ctxt, &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, start_pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32)?;

            // Check if logits are all zero
            let is_all_zero = logits.data().chunks_exact(4).all(|bytes| {
                let value = f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                value == 0.0
            });

            if is_all_zero {
                log::warn!("All logits are zero at index {}", index);

                // Retry up to 3 times
                let max_retries = 3;
                for retry in 1..=max_retries {
                    log::info!("Retrying ({}/{})", retry, max_retries);

                    match self.model.forward(&input, start_pos) {
                        Ok(logits) => {
                            let logits = match logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32) {
                                Ok(logits) => logits,
                                Err(e) => {
                                    log::error!("Error during logits processing: {}", e);
                                    return Err(anyhow::format_err!(
                                        "Failed to process logits after {} retries: {}",
                                        retry,
                                        e
                                    ));
                                }
                            };

                            let is_all_zero = logits.data().chunks_exact(4).all(|bytes| {
                                let value =
                                    f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                                value == 0.0
                            });

                            if !is_all_zero {
                                break;
                            }
                        }
                        Err(e) => {
                            log::error!("Error during retry: {}", e);
                            if retry == max_retries {
                                return Err(anyhow::format_err!(
                                    "Failed to generate logits after {} retries: {}",
                                    max_retries,
                                    e
                                ));
                            }
                        }
                    }

                    if retry == max_retries {
                        return Err(anyhow::format_err!(