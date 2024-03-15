use crate::scale_image;
use crate::stable_diffusion::SDConfig;
use crate::stable_diffusion::StableDiffusionVersion;
use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use image::ImageBuffer;
use image::Rgb;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub async fn sd_auto(
    config: SDConfig,
) -> Result<Vec<ImageBuffer<Rgb<u8>, Vec<u8>>>, anyhow::Error> {
    let client = Client::new();

    let model = match config.sd_ve