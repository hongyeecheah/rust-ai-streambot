/*
 * lib.rs
 * ------
 * Author: Chris Kennedy February @2024
 *
 * This file contains the main library for the stats and network capture modules
 * for RsLLM.
*/

pub mod args;
pub mod audio;
pub mod candle_metavoice;
pub mod candle_mistral;
pub mod mimic3_tts;
pub mod mpegts;
#[cfg(feature = "ndi")]
pub mod ndi;
pub mod network_capture;
pub mod openai_api;
pub mod openai_tts;
pub mod pipeline;
pub mod sd_automatic;
pub mod stable_diffusion;
pub mod stream_data;
pub mod system_stats;
pub mod twitch_client;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
pub use system_stats::{get_system_stats, SystemStats};
pub mod candle_gemma;
use image::{
    imageops::{resize, FilterType},
    ImageBuffer, Rgb, Rgba,
};
#[cfg(feature = "fonts")]
use imageproc::drawing::draw_text_mut;
#[cfg(feature = "fonts")]
use rusttype::{Font, Scale};
use std::io::Write;

#[derive(Debug)]
pub enum ApiError {
    E