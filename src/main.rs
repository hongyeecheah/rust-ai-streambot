
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