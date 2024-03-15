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

pub as