#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

#[cfg(feature = "metavoice")]
use anyhow::{Error, Result};
#[cfg(feature = "metavoice")]
use bytes::Bytes;
#[cfg(feature = "metavoice")]
use std::io::Cursor;
#[cfg(feature = "metavoice")]
use std::io::Write;

#[cfg(feature = "metavoice")]
use candle_transformers::generation::LogitsProcessor;
#[cfg(feature = "metavoice")]
use candle_transformers::models::encodec;
#[cfg(feature = "metavoice")]
use candle_transformers::models::metavoice::{adapters, gpt, tokenizers, transformer};
#[cfg(feature = "metavoice")]
use candle_transformers::models::quantized_metavoice::transformer as qtransformer;

#[cfg(feature = "metavoice")]
use candle_core::{DType, IndexOp, Tensor};
#[cfg(feature = "metavoice")]
use candle_nn::VarBuilder;
#[cfg(feature = "metavoice")]
use hf_hub::api::sync::Api;
#[cfg(feature = "metavoice")]
use rand::Rng;
#[cfg(feature = "metavoice")]
use rand::{distributions::Distribution, SeedableRng};

pub const ENCODEC_NTOKENS: u32 = 1024;

#[cfg(feature = "metavoice")]
enum Transformer {
    Normal(transformer::Model),
    Quantized(qtransformer::Model),
}

#[cfg(feature = "metavoice")]
pub async fn metavoice(prompt: String) -> Result<Bytes, Error> {
    use tracing_chrome::ChromeLayerBuilder;
    use tracing_subscriber::prelude::*;

    let show_status = false;
    let tracing = false;
    let cpu = false;
    let guidance_scale = 3.0;
    let temperature = 1.0;
    // Override seed for now
    let mut seed: Option<u64> = Some(299792458);
    let max_tokens = 2000;
    let first_stage_meta: Option<String> = None;
    let first_stage_weights: Option<String> = None;
    let second_stage_weights: Option<String> = None;
    let encodec_weights: Option<String> = None;
    let spk_emb: Option<String> = None;
    let dtype = DType::F32;
    let quantized = true;

    if seed.is_none() {
        seed = Some(rand::random());
    }

    let _guard = if tracing {
        let (chrome_layer, guard) = ChromeLayerBuilder::new().build();
        tracing_subscriber::registry().with(chrome_layer).init();
        Some(guard)
    } else {
        None
    };

    let device = candle_examples::device(cpu)?;
    let api = Api::new()?;
    let repo = api.model("lmz/candle-metavoice".to_string());
    let first_stage_meta = match &first_stage_meta {
        Some(w) => std::path::PathBuf::from(w),
        None => repo.get("first_stage.meta.json")?,
    };
    let first_stage_meta: serde_json::Value =
        serde_json::from_reader(&std::fs::File::open(first_stage_meta)?)?;
    let first_stage_tokenizer = match first_stage_meta.as_object() {
        None => anyhow::bail!("not a json object"),
        Some(j) => match j.get("tokenizer") {
            None => anyhow::bail!("no tokenizer key"),
            Some(j) => j,
        },
    };
    let f