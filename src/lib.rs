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
    Error(String),
    RequestError(reqwest::Error),
}

impl From<reqwest::Error> for ApiError {
    fn from(value: reqwest::Error) -> Self {
        ApiError::RequestError(value)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ApiError::Error(msg) => write!(f, "{}", msg),
            ApiError::RequestError(e) => write!(f, "Request error: {}", e),
        }
    }
}

/// Enum to determine the type of stats to fetch.
pub enum StatsType {
    System,
}

/// Fetches the requested stats and returns them as a JSON Value.
pub async fn get_stats_as_json(stats_type: StatsType) -> Value {
    match stats_type {
        StatsType::System => {
            let system_stats = get_system_stats();
            json!(system_stats)
        }
    }
}

// Function to get the current Unix timestamp in milliseconds
pub fn current_unix_timestamp_ms() -> Result<u64, &'static str> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .map_err(|_| "System time is before the UNIX epoch")
}

// Print a hexdump of the packet
pub fn hexdump(packet_arc: &Arc<Vec<u8>>, packet_offset: usize, packet_len: usize) {
    let packet = &packet_arc[packet_offset..packet_offset + packet_len];
    // print in rows of 16 bytes
    let mut packet_dump = String::new();
    for (i, chunk) in packet.iter().take(packet_len).enumerate() {
        if i % 16 == 0 {
            packet_dump.push_str(&format!("\n{:04x}: ", i));
        }
        packet_dump.push_str(&format!("{:02x} ", chunk));
    }
    println!(
        "--- Packet Offset {} Packet Length {} ---\n{}\n---",
        packet_offset, packet_len, packet_dump
    );
}

// return a string of the packet in hex plus ascii representation after each hex line (16 bytes) with a | delimiter
pub fn hexdump_ascii(packet: &[u8], packet_offset: usize, packet_len: usize) -> String {
    // Assuming packet_offset and packet_len are correctly calculated within the slice's bounds
    let packet = &packet[packet_offset..packet_offset + packet_len];
    let mut packet_dump = String::new();
    for (i, &chunk) in packet.iter().enumerate() {
        if i % 16 == 0 {
            packet_dump.push_str(&format!("\n{:04x}: ", i));
        }
        packet_dump.push_str(&format!("{:02x} ", chunk));
        if i % 16 == 15 || i == packet.len() - 1 {
            // Adjust for last line
            packet_dump.push_str(" | ");
            let start = if i % 16 == 15 { i - 15 } else { i / 16 * 16 };
            for &ch in &packet[start..=i] {
                if ch >= 32 && ch <= 126 {
                    packet_dump.push(ch as char);
                } else {
                    packet_dump.push('.');
                }
            }
        }
    }
    packet_dump
}

/// Remove all caps from the provided string.
pub fn adjust_caps(paragraph: &str) -> String {
    paragraph
        .split_whitespace()
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => {
                    f.to_uppercase().collect::<String>() + c.as_str().to_lowercase().as_str()
                }
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

/// Modifies the provided string if it exceeds 80 characters, splitting it according to specified delimiters,
/// and updates the `terminal_token_len` based on the operation performed.
///
/// # Arguments
///
/// * `received` - The string to potentially modify.
/// * `terminal_token_len` - The current length of the terminal token, to be updated.
pub fn handle_long_string(received: &str, terminal_token_len: &mut usize) {
    if *terminal_token_len >= 80 {
        std::io::stdout().flush().unwrap();

        // Initialize split position to the end of the string by default
        let mut split_pos = received.len();
        let mut found = false;
        for delimiter in ['\n', '.', ',', '?', '!'] {
            if let Some(pos) = received.find(delimiter) {
                // Adjust position to keep the delimiter with the first part, except for '\n'
                let end_pos = if delimiter == '\n' { pos } else { pos + 1 };
                split_pos = split_pos.min(end_pos);
                found = true;
                break;
            }
        }
        if split_pos == received.len() {
            if let Some(pos) = received.find(' ') {
                // Adjust position to keep the delimiter with the first part, except for '\n'
                let end_pos = pos + 1;
                split_pos = split_pos.min(end_pos);
                found = true;
            }
        }

        if found {
            let (first, second) = received.split_at(split_pos);
            print!("{}\n{}", first, second); // Use println! for simplicity to handle the newline
            *terminal_token_len = 0; //second.len(); // Update terminal_token_len with the length of the second part
        } else {
            print!("{}", received);
        }
        std::io::stdout().flush().unwrap();
    } else {
        print!("{}", received);
        std::io::stdout().flush().unwrap();
    }
}

/// Truncate the input text to the specified number of tokens.
/// If the number of tokens in the input text is less than or equal to the specified number of tokens,
/// the input text is returned as is. Otherwise, the input text is truncated to the specified number of tokens.
pub fn truncate_tokens(text: &str, max_tokens: usize) -> String {
    let mut tokens: Vec<String> = Vec::new();
    for token in text.split_whitespace() {
        if token.len() <= 4 {
            tokens.push(token.to_string());
        } else {
            let token_chars: Vec<char> = token.chars().collect();
            let chunks = token_chars.chunks(4);
            for chunk in chunks {
                let chunk_str: String = chunk.iter().collect();
                tokens.push(chunk_str);
            }
        }
    }

    if tokens.len() <= max_tokens {
        text.to_string()
    } else {
        tokens[..max_tokens].join(" ")
    }
}

pub fn count_tokens(text: &str) -> usize {
    let mut token_count = 0;
    for token in text.split_whitespace() {
        if token.len() <= 4 {
            token_count += 1;
        } else {
            let token_chars: Vec<char> = token.chars().collect();
            let chunks = token_chars.chunks(4);
            token_count += chunks.len();
        }
    }
   