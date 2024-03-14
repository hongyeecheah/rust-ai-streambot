
/*
    Image and Speech generation pipeline for NDI output
*/
use crate::adjust_caps;
use crate::args::Args;
#[cfg(feature = "ndi")]
use crate::audio::{mp3_to_f32, wav_to_f32};
#[cfg(feature = "metavoice")]
use crate::candle_metavoice::metavoice;
use crate::mimic3_tts::tts as mimic3_tts;
use crate::mimic3_tts::Request as Mimic3TTSRequest;
#[cfg(feature = "ndi")]
use crate::ndi::send_audio_samples_over_ndi;
#[cfg(feature = "ndi")]
use crate::ndi::send_images_over_ndi;
use crate::openai_tts::tts as oai_tts;
use crate::openai_tts::Request as OAITTSRequest;
use crate::openai_tts::Voice as OAITTSVoice;
use crate::sd_automatic::sd_auto;
use crate::stable_diffusion::{sd, SDConfig};
use crate::ApiError;
use image::ImageBuffer;
use image::Rgb;
use log::debug;

// Message Data for Image and Speech generation functions to use
#[derive(Clone)]
pub struct MessageData {
    pub paragraph: String,
    pub output_id: String,
    pub paragraph_count: usize,
    pub sd_config: SDConfig,
    pub mimic3_voice: String,
    pub subtitle_position: String,
    pub args: Args,
    pub shutdown: bool,
    pub last_message: bool,
}

// Function to process image generation
pub async fn process_image(mut data: MessageData) -> Vec<ImageBuffer<Rgb<u8>, Vec<u8>>> {
    // truncate tokens for sd_config.prompt
    data.sd_config.prompt = crate::truncate_tokens(&data.sd_config.prompt, data.args.sd_text_min);
    if data.args.sd_image {
        debug!("Generating images with prompt: {}", data.sd_config.prompt);

        let images = if data.args.sd_api {
            sd_auto(data.sd_config).await
        } else {
            sd(data.sd_config).await
        };

        match images {
            // Ensure `sd` function is async and await its result
            Ok(images) => {
                // Save images to disk
                if data.args.save_images {
                    for (index, image_bytes) in images.iter().enumerate() {
                        let image_file = format!(
                            "images/{}_{}_{}_.png",
                            data.output_id, data.paragraph_count, index
                        );
                        debug!(
                            "Image {} {}/{} saving to {}",
                            data.output_id, data.paragraph_count, index, image_file
                        );
                        image_bytes
                            .save(image_file)
                            .map_err(candle_core::Error::wrap)
                            .unwrap(); // And this as well
                    }
                }
                return images.clone();
            }
            Err(e) => {
                println!("");
                log::error!("Error generating images for {}: {:?}", data.output_id, e);
            }
        }