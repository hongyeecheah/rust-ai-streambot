
/*
 * RsLLM OpenAI API client
 * This program is a simple client for the OpenAI API. It sends a prompt to the API and prints the
 * response to the console.
 * The program is written in Rust and uses the reqwest crate for making HTTP requests.
 * The program uses the clap crate for parsing command line arguments.
 * The program uses the serde and serde_json crates for serializing and deserializing JSON.
 * The program uses the log crate for logging.
 * The program uses the tokio crate for asynchronous IO.
 * The program uses the chrono crate for working with dates and times.
 * The program uses the dotenv crate for reading environment variables from a .env file.
 *
 * Chris Kennedy (C) February 2024
 * MIT License
 *
*/

use clap::Parser;
use ctrlc;
use log::{debug, error, info};
use rsllm::args::Args;
use rsllm::candle_gemma::gemma;
use rsllm::candle_mistral::mistral;
use rsllm::clean_tts_input;
use rsllm::count_tokens;
use rsllm::handle_long_string;
use rsllm::network_capture::{network_capture, NetworkCapture};
use rsllm::openai_api::{format_messages_for_llm, stream_completion, Message, OpenAIRequest};
#[cfg(feature = "ndi")]
use rsllm::pipeline::send_to_ndi;
use rsllm::pipeline::{process_image, process_speech, MessageData, ProcessedData};
use rsllm::stable_diffusion::{SDConfig, StableDiffusionVersion};
use rsllm::stream_data::{
    get_pid_map, identify_video_pid, is_mpegts_or_smpte2110, parse_and_store_pat, process_packet,
    update_pid_map, Codec, PmtInfo, StreamData, Tr101290Errors, PAT_PID,
};
use rsllm::stream_data::{process_mpegts_packet, process_smpte2110_packet};
use rsllm::twitch_client::daemon as twitch_daemon;
use rsllm::{current_unix_timestamp_ms, hexdump, hexdump_ascii};
use rsllm::{get_stats_as_json, StatsType};
use serde_json::{self, json};
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use tokio::sync::mpsc::{self};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::Duration;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    // Read .env file
    dotenv::dotenv().ok();

    // Initialize logging
    let _ = env_logger::try_init();

    // Parse command line arguments
    let args = Args::parse();

    // Create an atomic bool to track if Ctrl+C is pressed
    let running_ctrlc = Arc::new(AtomicBool::new(true));
    let rctrlc = running_ctrlc.clone();

    // Set up the Ctrl+C handler
    ctrlc::set_handler(move || {
        println!("");
        println!(
            "Ctrl+C received, shutting down after all processes are stopped (Do not force quit)..."
        );
        rctrlc.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    // Set Rust log level with --loglevel if it is set
    let loglevel = args.loglevel.to_lowercase();
    match loglevel.as_str() {
        "error" => {
            log::set_max_level(log::LevelFilter::Error);
        }
        "warn" => {
            log::set_max_level(log::LevelFilter::Warn);
        }
        "info" => {
            log::set_max_level(log::LevelFilter::Info);
        }
        "debug" => {
            log::set_max_level(log::LevelFilter::Debug);
        }
        "trace" => {
            log::set_max_level(log::LevelFilter::Trace);
        }
        _ => {
            log::set_max_level(log::LevelFilter::Info);
        }
    }

    let system_message = Message {
        role: "system".to_string(),
        content: args.system_prompt.to_string(),
    };

    let processed_data_store: Arc<Mutex<HashMap<usize, ProcessedData>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Channels for image and speech tasks
    let (pipeline_task_sender, mut pipeline_task_receiver) =
        mpsc::channel::<MessageData>(args.pipeline_concurrency);

    // Channel to signal NDI is done
    #[cfg(feature = "ndi")]
    let (ndi_done_tx, mut ndi_done_rx) = mpsc::channel::<()>(1);

    let pipeline_sem = Arc::new(Semaphore::new(args.pipeline_concurrency));
    // Pipeline processing task for image and speech together as a single task
    // Pipeline processing task for image and speech together as a single task
    let pipeline_processing_task = {
        let pipeline_sem = Arc::clone(&pipeline_sem);
        let processed_data_store = processed_data_store.clone();
        // create a black frame image in the vec[] to use initially as last_images
        // Vec<ImageBuffer<Rgb<u8>, Vec<u8>>>
        let black_frame = image::ImageBuffer::from_fn(1920, 1080, |_, _| image::Rgb([0, 0, 0]));
        let last_images = Arc::new(Mutex::new(vec![black_frame.clone()]));
        tokio::spawn(async move {
            while let Some(message_data) = pipeline_task_receiver.recv().await {
                let processed_data_store = processed_data_store.clone();
                let message_data_clone = message_data.clone();
                let pipeline_sem = Arc::clone(&pipeline_sem);
                let last_images_clone = Arc::clone(&last_images);
                // channels to pass images back for the last_images vec
                let (image_tx, mut image_rx) =
                    mpsc::channel::<Vec<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>>>(100);
                let image_task = tokio::spawn(async move {
                    let _permit = pipeline_sem
                        .acquire()
                        .await
                        .expect("failed to acquire pipeline semaphore permit");

                    // Create a new black_frame for each iteration
                    let black_frame =
                        image::ImageBuffer::from_fn(1920, 1080, |_, _| image::Rgb([0, 0, 0]));

                    // check length of message_data, if it is less than 80 characters, use last_images
                    /*if message_data_clone.paragraph.len() < 80 {
                    let last_images = last_images_clone.lock().await;
                    let images = last_images.clone();
                    }*/

                    // process_image returns an empty vec if there are no images
                    let mut images = process_image(message_data_clone.clone()).await;

                    // check if image is all black
                    let mut all_black = true;
                    for img in images.iter() {
                        for pixel in img.pixels() {
                            if pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0 {
                                all_black = false;
                                break;
                            }
                        }
                    }
                    if all_black {
                        std::io::stdout().flush().unwrap();
                        println!("");
                        log::error!("Image is all black, skipping");
                    }

                    // Check if the processed images are empty
                    if images.is_empty() || all_black {
                        // If the processed images are empty, use the last_images
                        let last_images_guard = last_images_clone.lock().await;
                        if !last_images_guard.is_empty() {
                            images = last_images_guard.clone();
                            std::io::stdout().flush().unwrap();
                            println!("");
                            log::error!("Images is empty, using last images");
                        } else {
                            println!("");
                            log::error!("Last Images is empty, using black image");
                            images = vec![black_frame];
                        }
                    } else {
                        // If the processed images are not empty, update the last_images
                        let mut last_images_guard = last_images_clone.lock().await;
                        *last_images_guard = images.clone();
                    }

                    // send images to the image channel
                    let _ = image_tx.send(images.clone()).await;

                    // update image cache images
                    let speech_data = process_speech(message_data_clone.clone()).await;
                    let mut store = processed_data_store.lock().await;