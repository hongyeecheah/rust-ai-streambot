
#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

use anyhow::{Error as E, Result};
use safetensors::tensor::View;
use std::io::Write;
use tokio::sync::mpsc::{self, Sender};
use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use candle_transformers::models::mistral::{Config, Model as Mistral};
use candle_transformers::models::quantized_mistral::Model as QMistral;

use candle_core::{DType, Device, Tensor};
use candle_examples::token_output_stream::TokenOutputStream;
use candle_nn::VarBuilder;
use candle_transformers::generation::LogitsProcessor;
use hf_hub::{api::sync::Api, Repo, RepoType};
use log::{debug, info};
use std::sync::Arc;
use tokenizers::Tokenizer;
use tokio::sync::Mutex;

enum Model {
    Mistral(Mistral),
    Quantized(QMistral),
}