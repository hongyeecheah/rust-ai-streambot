/*
Implement the OpenAI API generically for any LLM following it
Chris Kennedy @2024 MIT license
*/

use bytes::Bytes;
use chrono::{TimeZone, Utc};
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::sync::mpsc::{self};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct OpenAIRequest<'a> {
    pub model: &'a str,
    pub messages: Vec<Message>,
    pub max_tokens: &'a usize,      // add this field to the request struct
    pub temperature: &'a f32,       // add this field to the request struct
    pub top_p: &'a f32,             // add this field to the request struct
    pub presence_penalty: &'a f32,  // add this field to the request struct
    pub frequency_penalty: &'a f32, // add this field to the request struct
    pub stream: &'a bool,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    role: Option<String>,
    created: Option<i64>,
    id: Option<String>,
    model: Option<String>,
    object: Option<String>,
    choices: Option<Vec<Choice>>,
    content: Option<String>,
    system_fingerprint: Option<String>,
}

#[derive(Deserialize)]
pub struct Choice {
    finish_reason: Option<String>,
    logprobs: Option<bool>,
    index: i32,
    delta: Delta, // Use Option to handle cases where it might be null or missing
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    content: Option<String>,
}

pub fn format_messages_for_llm(messages: Vec<Message>, chat_format: String) -> String {
    let mut formatted_history = String::new();
    // Begin/End Stream Tokens
    let eos_token = if chat_format == "llama2" { "</s>" } else { "" };
    let bos_token = if chat_format == "llama2" { "<s>" } else { "" };
    // Instruction Tokens
    let inst_token = if chat_format == "llama2" {
        "[INST]"
    } else if chat_format == "google" {
        "<start_of_turn>"
    } else if chat_format == "chatml" {
        "<im_start>"
    } else if chat_format == "vicuna" {
        ""
    } else {
        ""
    };
    let inst_end_token = if chat_format == "llama2" {
        "[/INST]"
    } else if chat_format == "google" {
        "<end_of_turn>"
    } else if chat_format == "chatml" {
        "<im_end>"
    } else if chat_format == "vicuna" {
        "\n"
    } else {
        ""
    };
    // Assistant Tokens
    let assist_token = if chat_format == "llama2" {
        ""
    } else if chat_format == "google" {
        "<start_of_turn>"
    } else if chat_format == "chatml" {
        "<im_start>"
    } else if chat_format == "vicuna" {
        ""
    } else {
        ""
    };
    let assist_end_token = if chat_format == "llama2" {
        ""
    } else if chat_format == "google" {
        "<end_of_turn>"
    } else if chat_format == "chatml" {
        "<im_end>"
    } else if chat_format == "vicuna" {
        "\n"
    } else {
        ""
    };
    // System Tokens
    let sys_token = if chat_format == "llama2" {
        "<<SYS>>"
    } else if chat_format == "google" {
        "<start_of_turn>"
    } else if chat_format == "chatml" {
        "<im_start>"
    } else if chat_format == "vicuna" {
        ""
    } else {
        ""
    };
    let sys_end_token = if chat_format == "llama2" {
        "<</SYS>>"
    } else if chat_format == "google" {
        "<end_of_turn>"
    } else if chat_format == "chatml" {
        "<im_end>"
    } else if chat_format == "vicuna" {
        "\n"
    } else {
        ""
    };
    // Names
    let sys_name = if chat_format == "llama2" {
        ""
    } else if chat_format == "google" {
        "model"
    } else if chat_format == "chatml" {
        "system"
    } else if chat_format == "vicuna" {
        "System: "
    } else {
        ""
    };
    let user_name = if chat_format == "ll