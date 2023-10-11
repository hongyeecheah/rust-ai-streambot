
#[cfg(not(feature = "fonts"))]
use crate::convert_rgb_to_rgba;
#[cfg(feature = "fonts")]
use crate::convert_rgb_to_rgba_with_text;
use image::{ImageBuffer, Rgb};
#[cfg(feature = "ndi")]
use ndi_sdk_rsllm::send::{SendColorFormat, SendInstance};
#[cfg(feature = "ndi")]
use ndi_sdk_rsllm::NDIInstance;
use once_cell::sync::Lazy;
use std::io::Result;
use std::sync::Mutex;

// Use Mutex to ensure thread-safety for NDIInstance and SendInstance
#[cfg(feature = "ndi")]
static NDI_INSTANCE: Lazy<Mutex<NDIInstance>> = Lazy::new(|| {
    let instance = ndi_sdk_rsllm::load().expect("Failed to construct NDI instance");
    Mutex::new(instance)
});

#[cfg(feature = "ndi")]
static NDI_SENDER: Lazy<Mutex<SendInstance>> = Lazy::new(|| {
    let instance = NDI_INSTANCE.lock().unwrap();
    let sender = instance
        .create_send_instance("RsLLM".to_string(), false, false)
        .expect("Expected sender instance to be created");
    Mutex::new(sender)
});

#[cfg(feature = "ndi")]
pub fn send_images_over_ndi(
    images: Vec<ImageBuffer<Rgb<u8>, Vec<u8>>>,
    subtitle: &str,
    font_size: f32,
    subtitle_position: &str,
) -> Result<()> {
    let mut sender = NDI_SENDER.lock().unwrap();

    for image_buffer in images {
        let width = image_buffer.width();
        let height = image_buffer.height();

        // adjust height depending on subtitle_postion as top, center, bottom with respect to the image height
        #[cfg(feature = "fonts")]
        let mut subtitle_height = height as i32 - (height as i32 / 3);
        #[cfg(feature = "fonts")]
        if subtitle_position == "top" {
            subtitle_height = 10;
        } else if subtitle_position == "mid-top" {