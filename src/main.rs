
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

                    match store.entry(message_data_clone.paragraph_count) {
                        std::collections::hash_map::Entry::Vacant(e) => {
                            e.insert(ProcessedData {
                                paragraph: message_data_clone.paragraph.clone(),
                                image_data: Some(images),
                                audio_data: Some(speech_data),
                                paragraph_count: message_data_clone.paragraph_count,
                                subtitle_position: message_data_clone.subtitle_position.clone(),
                                time_stamp: 0,
                                shutdown: message_data_clone.shutdown.clone(),
                                completed: true,
                                last_message: message_data_clone.last_message.clone(),
                            });
                        }
                        std::collections::hash_map::Entry::Occupied(mut e) => {
                            let entry = e.get_mut();
                            entry.image_data = Some(images);
                            entry.audio_data = Some(speech_data);
                            entry.completed = true;
                        }
                    }
                });

                // wait for images and collect any in and put into the last_images vec
                if let Some(images) = image_rx.recv().await {
                    let mut last_images = last_images.lock().await;
                    *last_images = images;
                }

                // wait for the image task to finish
                image_task.await.unwrap();

                // Check if this is the last message
                if message_data.last_message {
                    std::io::stdout().flush().unwrap();
                    info!(
                        "Pipeline processing task: Last message processed {}",
                        message_data.paragraph_count
                    );
                }

                // check if shutdown is requested from the message shutdown flag
                if message_data.shutdown {
                    std::io::stdout().flush().unwrap();
                    info!("Shutdown requested from message data for pipeline processing task.");
                    break;
                }
            }
        })
    };

    // NDI sync task
    #[cfg(feature = "ndi")]
    let processed_data_store_for_ndi = processed_data_store.clone();
    #[cfg(feature = "ndi")]
    let args_for_ndi = args.clone();

    #[cfg(feature = "ndi")]
    let running_processor_ndi = Arc::new(AtomicBool::new(true));
    #[cfg(feature = "ndi")]
    let running_processor_ndi_clone = running_processor_ndi.clone();
    #[cfg(feature = "ndi")]
    let ndi_sync_task = tokio::spawn(async move {
        let mut current_key = 0;
        let mut max_key = 0;

        while running_processor_ndi_clone.load(Ordering::SeqCst) {
            let mut data = {
                let store = processed_data_store_for_ndi.lock().await;
                store.get(&current_key).cloned()
            };

            if let Some(ref mut data) = data {
                if data.completed {
                    // Update max_key if necessary
                    max_key = max_key.max(data.paragraph_count);

                    // check if we are reset to paragraph count 1, if so, reset the max_key and current key back to 1 and set as last_message
                    if data.paragraph_count == 0 && current_key > 0 {
                        max_key = 0;
                        current_key = 0;
                        data.last_message = true;
                    }

                    // Check if this is the last message and send the NDI done signal
                    if data.last_message {
                        std::io::stdout().flush().unwrap();
                        debug!(
                            "NDI sync task: Last message {} processed for key {}, sending done signal.",
                            data.paragraph_count, current_key
                        );
                        // Send NDI done signal
                        if let Err(e) = ndi_done_tx.send(()).await {
                            error!("Failed to send NDI done signal: {}", e);
                        }
                        std::io::stdout().flush().unwrap();
                        debug!(
                            "Sent NDI Sending done signal for {} key {}.",
                            data.paragraph_count, current_key
                        );
                    }

                    // Send to NDI
                    #[cfg(feature = "ndi")]
                    send_to_ndi(data.clone(), &args_for_ndi).await;
                    {
                        let mut store = processed_data_store_for_ndi.lock().await;
                        store.remove(&current_key);
                    }
                    current_key += 1;
                } else {
                    std::io::stdout().flush().unwrap();
                    debug!(
                        "NDI sync task: Message {} data not completed for key {}",
                        data.paragraph_count, current_key
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                }
            } else {
                std::io::stdout().flush().unwrap();
                debug!("NDI sync task: No data found for key {}", current_key);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                // If the current key is not found, check if it's less than the max key
                /*if current_key < max_key {
                    // If the current key is less than the max key, increment the current key and continue
                    log::error!(
                        "NDI sync task: Current key {} is less than max key {}",
                        current_key,
                        max_key
                    );
                    current_key += 1;
                } else {
                    // If the current key is equal to or greater than the max key, sleep and continue
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }*/
            }

            // SHUTDOWN Signal
            if data.is_some() && data.as_ref().unwrap().shutdown {
                running_processor_ndi_clone.store(false, Ordering::SeqCst);
                std::io::stdout().flush().unwrap();
                info!("Shutting down NDI sync task on shutdown signal.");
                break;
            }
        }

        // exit the loop
        std::io::stdout().flush().unwrap();
        info!("Exiting NDI sync task.");
        std::process::exit(0);
    });

    let mut llm_host = args.llm_host.clone();
    if args.use_openai {
        // set the llm_host to the openai api
        llm_host = "https://api.openai.com".to_string();
    }

    // start time
    let start_time = current_unix_timestamp_ms().unwrap_or(0);
    let mut total_paragraph_count = 0;

    // Perform TR 101 290 checks
    let mut tr101290_errors = Tr101290Errors::new();
    // calculate read size based on batch size and packet size
    let read_size: i32 =
        (args.packet_size as i32 * args.pcap_batch_size as i32) + args.payload_offset as i32; // pcap read size
    let mut is_mpegts = true; // Default to true, update based on actual packet type

    let (ptx, mut prx) = mpsc::channel::<Arc<Vec<u8>>>(args.pcap_channel_size);
    let (batch_tx, mut batch_rx) = mpsc::channel::<String>(args.pcap_channel_size); // Channel for passing processed packets to main logic
    let mut network_capture_config = NetworkCapture {
        running: Arc::new(AtomicBool::new(true)),
        dpdk: false,
        use_wireless: args.use_wireless,
        promiscuous: args.promiscuous,
        immediate_mode: args.immediate_mode,
        source_protocol: Arc::new(args.source_protocol.to_string()),
        source_device: Arc::new(args.source_device.to_string()),
        source_ip: Arc::new(args.source_ip.to_string()),
        source_port: args.source_port,
        read_time_out: 60_000,
        read_size,
        buffer_size: args.buffer_size,
        pcap_stats: args.pcap_stats,
        debug_on: args.hexdump,
        capture_task: None,
    };

    // Initialize messages with system_message outside the loop
    let mut messages = vec![system_message.clone()];

    // Initialize the network capture if ai_network_stats is true
    if args.ai_network_stats {
        network_capture(&mut network_capture_config, ptx);
    }

    let running_processor_network = Arc::new(AtomicBool::new(true));
    let running_processor_network_clone = running_processor_network.clone();

    let processing_handle = tokio::spawn(async move {
        let mut decode_batch = Vec::new();
        let mut video_pid: Option<u16> = Some(0xFFFF);
        let mut video_codec: Option<Codec> = Some(Codec::NONE);
        let mut current_video_frame = Vec::<StreamData>::new();
        let mut pmt_info: PmtInfo = PmtInfo {
            pid: 0xFFFF,
            packet: Vec::new(),
        };

        let mut packet_last_sent_ts = Instant::now();
        let mut count = 0;
        while running_processor_network_clone.load(Ordering::SeqCst) {
            if args.ai_network_stats {
                debug!("Capturing network packets...");
                while let Some(packet) = prx.recv().await {
                    count += 1;
                    debug!(
                        "#{} --- Received packet with size: {} bytes",
                        count,
                        packet.len()
                    );

                    // Check if chunk is MPEG-TS or SMPTE 2110
                    let chunk_type = is_mpegts_or_smpte2110(&packet[args.payload_offset..]);
                    if chunk_type != 1 {
                        if chunk_type == 0 {
                            hexdump(&packet, 0, packet.len());
                            error!("Not MPEG-TS or SMPTE 2110");
                        }
                        is_mpegts = false;
                    }

                    // Process the packet here
                    let chunks = if is_mpegts {
                        process_mpegts_packet(
                            args.payload_offset,
                            packet,
                            args.packet_size,
                            start_time,
                        )
                    } else {
                        process_smpte2110_packet(
                            args.payload_offset,
                            packet,
                            args.packet_size,
                            start_time,
                            false,
                        )
                    };

                    // Process each chunk
                    for mut stream_data in chunks {
                        // check for null packets of the pid 8191 0x1FFF and skip them
                        if stream_data.pid >= 0x1FFF {
                            debug!("Skipping null packet");
                            continue;
                        }

                        if args.hexdump {
                            hexdump(
                                &stream_data.packet,
                                stream_data.packet_start,
                                stream_data.packet_len,
                            );
                        }

                        // Extract the necessary slice for PID extraction and parsing
                        let packet_chunk = &stream_data.packet[stream_data.packet_start
                            ..stream_data.packet_start + stream_data.packet_len];

                        if is_mpegts {
                            let pid = stream_data.pid;
                            // Handle PAT and PMT packets
                            match pid {
                                PAT_PID => {
                                    debug!("ProcessPacket: PAT packet detected with PID {}", pid);
                                    pmt_info = parse_and_store_pat(&packet_chunk);
                                    // Print TR 101 290 errors
                                    if args.show_tr101290 {
                                        info!("STATUS::TR101290:ERRORS: {}", tr101290_errors);
                                    }
                                }
                                _ => {
                                    // Check if this is a PMT packet
                                    if pid == pmt_info.pid {
                                        debug!(
                                            "ProcessPacket: PMT packet detected with PID {}",
                                            pid
                                        );
                                        // Update PID_MAP with new stream types
                                        update_pid_map(&packet_chunk, &pmt_info.packet);
                                        // Identify the video PID (if not already identified)
                                        if let Some((new_pid, new_codec)) =
                                            identify_video_pid(&packet_chunk)
                                        {
                                            if video_pid.map_or(true, |vp| vp != new_pid) {
                                                video_pid = Some(new_pid);
                                                info!(
                                                    "STATUS::VIDEO_PID:CHANGE: to {}/{} from {}/{}",
                                                    new_pid,
                                                    new_codec.clone(),
                                                    video_pid.unwrap(),
                                                    video_codec.unwrap()
                                                );
                                                video_codec = Some(new_codec.clone());
                                                // Reset video frame as the video stream has changed
                                                current_video_frame.clear();
                                            } else if video_codec != Some(new_codec.clone()) {
                                                info!(
                                                    "STATUS::VIDEO_CODEC:CHANGE: to {} from {}",
                                                    new_codec,
                                                    video_codec.unwrap()
                                                );
                                                video_codec = Some(new_codec);
                                                // Reset video frame as the codec has changed
                                                current_video_frame.clear();
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Check for TR 101 290 errors
                        process_packet(
                            &mut stream_data,
                            &mut tr101290_errors,
                            is_mpegts,
                            pmt_info.pid,
                        );
                        count += 1;

                        decode_batch.push(stream_data);
                    }

                    // check if it is 60 seconds since the last packet was sent
                    let last_packet_sent = packet_last_sent_ts.elapsed().as_secs();

                    // If the batch is full, process it
                    if args.poll_interval == 0
                        || (last_packet_sent > (args.poll_interval / 1000)
                            && decode_batch.len() > args.ai_network_packet_count)
                    {
                        let mut network_packet_dump: String = String::new();