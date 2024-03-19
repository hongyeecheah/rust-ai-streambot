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

    let model = match config.sd_version {
        StableDiffusionVersion::Custom => config.custom_model.as_deref().unwrap_or("sd_xl_turbo_1.0.safetensors"),
        StableDiffusionVersion::V1_5 => "v1-5-pruned-emaonly.ckpt",
        StableDiffusionVersion::V2_1 => "v2-1_768-ema-pruned.ckpt",
        StableDiffusionVersion::Xl => "stabilityai/stable-diffusion-xl-1024-1.0.ckpt",
        StableDiffusionVersion::Turbo => "sd_xl_turbo_1.0.safetensors",
    };

    let payload = AutomaticPayload {
        prompt: config.prompt,
        negative_prompt: config.uncond_prompt,
        steps: config.n_steps.unwrap_or(20),
        width: config.width.unwrap_or(512),
        height: config.height.unwrap_or(512),
        cfg_scale: config.guidance_scale.unwrap_or(7.5),
        sampler_index: "Euler".to_string(),
        seed: config.seed.unwrap_or_else(rand::random) as u64,
        n_iter: config.num_samples,
        batch_size: 1,
        override_settings: OverrideSettings {
            sd_model_checkpoint: model.to_string(),
        },
    };

    let response = client
        .post("http://127.0.0.1:7860/sdapi/v1/txt2img")
   