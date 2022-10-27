
use clap::Parser;

/// RScap Probe Configuration
#[derive(Parser, Debug, Clone)]
#[clap(
    author = "Chris Kennedy",
    version = "0.5.13",
    about = "Rust AI Stream Analyzer Twitch Bot"
)]
pub struct Args {
    /// System prompt
    #[clap(
        long,
        env = "SYSTEM_PROMPT",
        default_value = "You are RsLLM the AI Analyzer. You carry on conversations and help people with their tasks. You are very friendly and polite. You are a good listener and always try to help people feel better.",
        help = "System prompt"
    )]
    pub system_prompt: String,

    /// Prompt
    #[clap(
        long,
        env = "QUERY",
        default_value = "",
        help = "Query to generate completions for, empty is interactive mode."
    )]
    pub query: String,

    /// Chat Format - LLM chat format to use, llama2, chatml, gemma, ""
    #[clap(
        long,
        env = "CHAT_FORMAT",
        default_value = "",
        help = "Chat Format - LLM chat format to use, llama2, chatml, gemma, \"\""
    )]
    pub chat_format: String,

    /// Temperature
    #[clap(
        long,
        env = "TEMPERATURE",
        default_value = "0.8",
        help = "Temperature for LLM sampling, 0.0 to 1.0, it will cause the LLM to generate more random outputs. 0.0 is deterministic, 1.0 is maximum randomness."
    )]
    pub temperature: f32,

    /// Model ID - for gemma 2b or 7b, mistral has various options too
    #[clap(
        long,
        env = "MODEL_ID",
        default_value = "auto",
        help = "Model ID - model path on huggingface or 7b / 2b for gemma"
    )]
    pub model_id: String,

    /// Quantized bool
    #[clap(
        long,
        env = "QUANTIZED",
        default_value = "false",
        help = "Quantized, it will use a quantized LLM to generate output faster on CPUs or GPUs."
    )]
    pub quantized: bool,

    /// Top P
    #[clap(
        long,
        env = "TOP_P",
        default_value = "1.0",
        help = "Top P sampling, 0.0 to 1.0."
    )]
    pub top_p: f32,

    /// Presence Penalty
    #[clap(
        long,
        env = "PRESENCE_PENALTY",
        default_value = "0.0",
        help = "Presence Penalty, it will cause the LLM to generate more diverse outputs. 0.0 is deterministic, 1.0 is maximum randomness."
    )]
    pub presence_penalty: f32,

    /// Frequency Penalty
    #[clap(
        long,
        env = "FREQUENCY_PENALTY",
        default_value = "0.0",
        help = "Frequency Penalty, it will cause the LLM to generate more diverse outputs. 0.0 is deterministic, 1.0 is maximum randomness."
    )]
    pub frequency_penalty: f32,

    /// Max Tokens
    #[clap(
        long,
        env = "MAX_TOKENS",
        default_value = "800",