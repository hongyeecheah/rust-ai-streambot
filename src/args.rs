
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
        help = "Max Tokens, 1 to N."
    )]
    pub max_tokens: i32,

    /// Model
    #[clap(
        long,
        env = "MODEL",
        default_value = "no-model-specified",
        help = "OpenAI LLM Model (N/A with local Llama2 based LLM)"
    )]
    pub model: String,

    /// LLM Host url with protocol, host, port,  no path
    #[clap(
        long,
        env = "LLM_HOST",
        default_value = "http://127.0.0.1:8080",
        help = "LLM Host url with protocol, host, port,  no path"
    )]
    pub llm_host: String,

    /// LLM Url path
    #[clap(
        long,
        env = "LLM_PATH",
        default_value = "/v1/chat/completions",
        help = "LLM Url path for completions."
    )]
    pub llm_path: String,

    /// LLM History size
    #[clap(
        long,
        env = "LLM_HISTORY_SIZE",
        default_value = "16768",
        help = "LLM History size (0 is unlimited)."
    )]
    pub llm_history_size: usize,

    /// Clear History - clear the history of the LLM each iteration
    #[clap(
        long,
        env = "CLEAR_HISTORY",
        default_value = "false",
        help = "Clear History - clear the history of the LLM each iteration."
    )]
    pub no_history: bool,

    /// Interactive mode - command line input
    #[clap(
        long,
        env = "INTERACTIVE",
        default_value = "false",
        help = "Interactive mode - command line input."
    )]
    pub interactive: bool,

    /// Don't stream output
    #[clap(
        long,
        env = "NO_STREAM",
        default_value = "false",
        help = "Don't stream output, wait for all completions to be generated before returning."
    )]
    pub no_stream: bool,

    /// Safety feature for using openai api and confirming you understand the risks
    #[clap(
        long,
        env = "USE_OPENAI",
        default_value = "false",
        help = "Safety feature for using openai api and confirming you understand the risks, you must also set the OPENAI_API_KEY, this will set the llm-host to api.openai.com."
    )]
    pub use_openai: bool,

    /// MetaVoice as text to speech
    #[clap(
        long,
        env = "METAVOICE_TTS",
        default_value = "false",
        help = "MetaVoice as text to speech."
    )]
    pub metavoice_tts: bool,

    /// OAI_TTS as text to speech from openai
    #[clap(
        long,
        env = "OAI_TTS",
        default_value = "false",
        help = "OAI_TTS as text to speech from openai."
    )]
    pub oai_tts: bool,

    /// MIMIC3_TTS as text to speech from openai
    #[clap(
        long,
        env = "MIMIC3_TTS",
        default_value = "false",
        help = "MIMIC3_TTS as text from mimic3-server local API."
    )]
    pub mimic3_tts: bool,

    /// MIMIC3_VOICE voice model via text string to use for mimic3 tts, en_US/vctk_low#p326 is a good male voice
    #[clap(
        long,
        env = "MIMIC3_VOICE",
        default_value = "en_US/vctk_low#p303",
        help = "MIMIC3_VOICE voice model via text string to use for mimic3 tts. Use en_US/vctk_low#p326 for a male voice, default is female."
    )]
    pub mimic3_voice: String,

    /// TTS text to speech enable
    #[clap(
        long,
        env = "TTS_ENABLE",
        default_value = "false",
        help = "TTS text to speech enable."
    )]
    pub tts_enable: bool,

    /// audio chunk size
    #[clap(
        long,
        env = "AUDIO_CHUNK_SIZE",
        default_value = "1.0",
        help = "audio chunk size in seconds for text to speech."
    )]
    pub audio_chunk_size: f32,

    /// Pipeline concurrency - max concurrent pipeline tasks
    #[clap(
        long,
        env = "PIPELINE_CONCURRENCY",
        default_value = "1",
        help = "Pipeline concurrency - max concurrent pipeline tasks."
    )]
    pub pipeline_concurrency: usize,

    /// debug inline on output (can mess up the output) as a bool
    #[clap(
        long,
        env = "DEBUG_INLINE",
        default_value = "false",
        help = "debug inline on output (can mess up the output) as a bool."
    )]
    pub debug_inline: bool,

    /// Show output errors
    #[clap(
        long,
        env = "SHOW_OUTPUT_ERRORS",
        default_value = "false",
        help = "Show LLM output errors which may mess up the output and niceness if packet loss occurs."
    )]
    pub show_output_errors: bool,

    /// Monitor system stats
    #[clap(
        long,
        env = "AI_OS_STATS",
        default_value = "false",
        help = "Monitor system stats."
    )]
    pub ai_os_stats: bool,

    /// run as a daemon monitoring the specified stats
    #[clap(
        long,
        env = "DAEMON",
        default_value = "false",
        help = "run as a daemon monitoring the specified stats."
    )]
    pub daemon: bool,

    /// AI Network Stats
    #[clap(
        long,
        env = "AI_NETWORK_STATS",
        default_value = "false",
        help = "Monitor network stats."
    )]
    pub ai_network_stats: bool,

    /// AI Network Packets - also send all the packets not jsut the pidmap stats
    #[clap(
        long,
        env = "AI_NETWORK_PACKETS",
        default_value = "false",
        help = "Monitor network packets."
    )]
    pub ai_network_packets: bool,

    /// AI Network Full Packet Hex Dump
    #[clap(
        long,
        env = "AI_NETWORK_HEXDUMP",
        default_value = "false",
        help = "Monitor network full packet hex dump."
    )]
    pub ai_network_hexdump: bool,

    /// AI Network Packet Count
    #[clap(
        long,
        env = "AI_NETWORK_PACKET_COUNT",
        default_value_t = 100,
        help = "AI Network Packet Count."
    )]
    pub ai_network_packet_count: usize,

    /// PCAP output capture stats mode
    #[clap(
        long,
        env = "PCAP_STATS",
        default_value_t = false,
        help = "PCAP output capture stats mode."
    )]
    pub pcap_stats: bool,

    /// Sets the batch size
    #[clap(
        long,
        env = "PCAP_BATCH_SIZE",
        default_value_t = 7,
        help = "Sets the batch size."
    )]
    pub pcap_batch_size: usize,

    /// Sets the payload offset
    #[clap(
        long,
        env = "PAYLOAD_OFFSET",
        default_value_t = 42,
        help = "Sets the payload offset."
    )]
    pub payload_offset: usize,

    /// Sets the packet size
    #[clap(
        long,
        env = "PACKET_SIZE",
        default_value_t = 188,
        help = "Sets the packet size."
    )]
    pub packet_size: usize,

    /// Sets the pcap buffer size
    #[clap(long, env = "BUFFER_SIZE", default_value_t = 1 * 1_358 * 1_000, help = "Sets the pcap buffer size, default is 1 * 1_358 * 1_000.")]
    pub buffer_size: i64,

    /// Sets the read timeout
    #[clap(
        long,
        env = "READ_TIME_OUT",
        default_value_t = 300_000,
        help = "Sets the read timeout."
    )]
    pub read_time_out: i32,

    /// Sets the source device
    #[clap(
        long,
        env = "SOURCE_DEVICE",
        default_value = "",
        help = "Sets the source device for pcap capture."
    )]
    pub source_device: String,

    /// Sets the source IP
    #[clap(
        long,
        env = "SOURCE_IP",
        default_value = "224.0.0.200",
        help = "Sets the source IP to capture for pcap."
    )]
    pub source_ip: String,

    /// Sets the source protocol
    #[clap(
        long,
        env = "SOURCE_PROTOCOL",
        default_value = "udp",
        help = "Sets the source protocol to capture for pcap."
    )]
    pub source_protocol: String,

    /// Sets the source port
    #[clap(
        long,
        env = "SOURCE_PORT",
        default_value_t = 10_000,
        help = "Sets the source port to capture for pcap."
    )]
    pub source_port: i32,

    /// Sets if wireless is used
    #[clap(
        long,
        env = "USE_WIRELESS",
        default_value_t = false,
        help = "Sets if wireless is used."
    )]
    pub use_wireless: bool,

    /// Use promiscuous mode
    #[clap(
        long,
        env = "PROMISCUOUS",
        default_value_t = false,
        help = "Use promiscuous mode for network capture."
    )]