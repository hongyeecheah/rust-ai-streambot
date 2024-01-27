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
    let user_name = if chat_format == "llama2" {
        ""
    } else if chat_format == "google" {
        "user"
    } else if chat_format == "chatml" {
        "user"
    } else if chat_format == "vicuna" {
        "User: "
    } else {
        ""
    };
    let assist_name = if chat_format == "llama2" {
        ""
    } else if chat_format == "google" {
        "model"
    } else if chat_format == "chatml" {
        "assistant"
    } else if chat_format == "vicuna" {
        "Assistant: "
    } else {
        ""
    };

    for (index, message) in messages.iter().enumerate() {
        // check if last message, safely get if this is the last message
        let is_last = index == messages.len() - 1;
        match message.role.as_str() {
            // remove <|im_end|> from anywhere in message
            "system" => {
                let message_content = message.content.replace("<|im_end|>", "");
                formatted_history += &format!(
                    "{}{}{} {}{}{}\n",
                    bos_token, sys_token, sys_name, message_content, sys_end_token, eos_token
                );
            }
            "user" => {
                // Assuming user messages should be formatted as instructions
                let message_content = message.content.replace("<|im_end|>", "");
                formatted_history += &format!(
                    "{}{}{} {}{}\n",
                    bos_token, inst_token, user_name, message_content, inst_end_token
                );
            }
            "assistant" => {
                // Close the instruction tag for user/system messages and add the assistant's response
                let message_content = message.content.replace("<|im_end|>", "");
                if is_last {
                    formatted_history += &format!(
                        "{}{}{} {}\n",
                        bos_token, assist_token, assist_name, message_content
                    );
                } else {
                    formatted_history += &format!(
                        "{}{}{} {}{}{}\n",
                        bos_token,
                        assist_token,
                        assist_name,
                        message_content,
                        assist_end_token,
                        eos_token
                    );
                }
            }
            _ => {}
        }
    }

    //formatted_history += "Instructions: Use the previous converation between you the assitant and the user as context and to answer the last question asked by the User as the assitant.\nAssistant:";

    formatted_history
}

/*
 * {"choices":[{"finish_reason":"stop","index":0,"message":{"content":"The Los Angeles Dodgers won
 * the World Series in 2020. They defeated the Tampa Bay Rays in six
 * games.","role":"assistant"}}],"created":1706900958,"id":"chatcmpl-8jqjxqYj1IkKixqlHVvmTyJynoPOjaoA","model":"gpt-3.5-turbo","object":"chat.completion","usage":{"completion_tokens":30,"prompt_tokens":62,"total_tokens":92}}
 */

pub async fn stream_completion(
    open_ai_request: OpenAIRequest<'_>,
    openai_key: &str,
    llm_host: &str,
    llm_path: &str,
    debug_inline: bool,
    show_output_errors: bool,
    external_sender: tokio::sync::mpsc::Sender<String>,
) {
    let client = Client::new();

    // measure messages member size of the content member of each pair of the messages array
    let mut prompt_token_count = 0;
    for message in &open_ai_request.messages {
        prompt_token_count += message.content.split_whitespace().count();
    }

    let start_time = Instant::now();
    let response = client
        .post(format!("{}{}", llm_host, llm_path))
        .header("Authorization", format!("Bearer {}", openai_key))
        .json(&open_ai_request)
        .send()
        .await;

    // handle errors
    let mut response = match response {
        Ok(resp) => resp,
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };

    let mut token_count = 0;
    let mut byte_count = 0;
    let mut loop_count = 0;

    if !open_ai_request.stream {
        info!("Response status: {}", response.status());
        debug!("Headers: {:#?}", response.headers());
        let text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                eprintln!("Failed to get response text: {}", e);
                return;
            }
        };
        println!("\nLLM Response:\n  {}\n---\n", text);
        // send back over mpsc channel
        if let Err(e) = external_sender.send(text).await {
            eprintln!("Failed to send text over mpsc channel: {}", e);
            return;
        }
    } else {
        // Create an mpsc channel
        let (tx, mut rx) = mpsc::channel::<Bytes>(32);
        let (etx, mut erx) = mpsc::channel::<String>(32);

        let headers = response.headers().clone(); // Clone the headers
        let status = response.status(); // Copy the status as well since it's Copy

        // loop through the chunks
        // Spawn a new task for each chunk to process it asynchronously
        let worker = tokio::spawn(async move {
            let mut first_run = true;
            let mut add_newline = false;
            let mut add_space = false;
            while let Some(chunk) = rx.recv().await {
                loop_count += 1;

                if first_run {
                    // print headers properly without causing a borrow error
                    debug!("Headers: {:#?}", headers);
                    info!("Response status: {}", status);
                }

                first_run = false;

                debug!("#{} LLM Result Chunk: {:#?}\n", loop_count, chunk);
                let chunk_vec = Vec::from(chunk.as_ref());
                let chunk_str = match String::from_utf8(chunk_vec).ok() {
                    Some(s) => s,
                    None => {
                        error!(
                            "Invalid UTF-8 sequence, skipping chunk. {}/{:?}",
                            chunk.len(),
                            chunk
                        );
                        continue;
                    } // skip non-UTF-8 chunks
                };

                // Splitting the chunk based on "data: " prefix to handle multiple JSON blobs
                let json_blobs: Vec<&str> = chunk_str.split("\ndata: ").collect();
                let mut blob_count = 0;

                for json_blob in json_blobs.iter() {
                    blob_count += 1;
                    debug!("Json Blob: {}/{} - {}", loop_count, blob_count, json_blob);
                    if json_blob.is_empty() || *json_blob == "\n" {
                        debug!("Empty line in response chunks.");
                        continue;
                    }

                    if json_blob == &"[DONE]" {
                        info!("End of response chunks.\n");
                        break;
                    }

                    // Confirm we have a '{' at the start, or find the offset of first '{' character
                    let offset = json_blob.find('{').unwrap_or(0);
                    let response_json = &json_blob[offset..];

                    if response_json.is_empty() {
                        error!("Invalid response chunk:\n - '{}'", json_blob);
                        continue;
                    }

                    debug!("Chunk #{} response: '{}'", loop_count, response_json);

                    match serde_json::from_str::<OpenAIResponse>(response_json) {
                        Ok(res) => {
                            let content = match &res.content {
                                Some(content) => content,
                                None => "",
                            };

                            if !content.is_empty() {
                                println!("LLM Content Response: {}", content);
                            }

                            // if res.content exists then continue to the next chunk
                            if res.content.is_some() {
                                continue;
                            }

                            // Assume `res` is an instance of `OpenAIResponse` you've deserialized
                            let choices = &res.choices.unwrap_or_else(|| {
                                error!("No choices found in response.");
                                Vec::new() // Provide a default value that matches the expected type
                            });

                            let role = match res.role {
                                Some(role) => role,
                                None => "unknown".to_string(),
                            };

                            if let Some(choice) = choices.get(0) {
                                // check if we got the created date from res.created, if so convert it to naivedatatime for usage else use a default value
                                let created_date = match res.created {
                                    Some(created_timestamp) => {
                                        // Convert the timestamp to UTC DateTime first, then to NaiveDateTime.
                                        let naive_datetime = Utc
                                            .timestamp_opt(created_timestamp, 0)
                                            .single() // This attempts to resolve the Option<T>
                                            .map(|dt| dt.naive_utc()) // Convert DateTime<Utc> to NaiveDateTime
                                        