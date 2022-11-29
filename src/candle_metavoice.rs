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
    use tracing_chrome: