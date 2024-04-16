
use crate::args::Args;
use crate::candle_gemma::gemma;
use crate::candle_mistral::mistral;
use anyhow::Result;
use rand::Rng;
use rusqlite::{params, Connection};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{self};

pub async fn daemon(
    nick: String,
    token: String,
    channel: Vec<String>,
    running: Arc<AtomicBool>,
    twitch_tx: mpsc::Sender<String>,
    args: Args,
) -> Result<()> {
    let credentials = match Some(nick).zip(Some(token)) {
        Some((nick, token)) => tmi::client::Credentials::new(nick, token),
        None => tmi::client::Credentials::anon(),
    };

    let channels = channel
        .into_iter()
        .map(tmi::Channel::parse)
        .collect::<Result<Vec<_>, _>>()?;

    log::info!("Connecting as {}", credentials.nick);
    let mut client = tmi::Client::builder()
        .credentials(credentials)
        .connect()
        .await?;

    client.join_all(&channels).await?;
    log::info!("Joined the following channels: {}", channels.join(", "));

    run(client, channels, running, twitch_tx, args).await
}

async fn run(
    mut client: tmi::Client,
    channels: Vec<tmi::Channel>,
    running: Arc<AtomicBool>,
    twitch_tx: mpsc::Sender<String>,
    args: Args,
) -> Result<()> {
    // create a semaphore so no more than one message is sent to the AI at a time
    let semaphore = tokio::sync::Semaphore::new(args.twitch_llm_concurrency as usize);
    while running.load(Ordering::SeqCst) {
        let msg = client.recv().await?;

        match msg.as_typed()? {
            tmi::Message::Privmsg(msg) => {
                // acquire the semaphore to send a message to the AI
                let _chat_lock = semaphore.acquire().await.unwrap();
                on_msg(&mut client, msg, &twitch_tx, args.clone()).await?
            }
            tmi::Message::Reconnect => {
                client.reconnect().await?;
                client.join_all(&channels).await?;
            }
            tmi::Message::Ping(ping) => client.pong(&ping).await?,
            _ => {}
        };
    }
    Ok(())
}

async fn on_msg(
    client: &mut tmi::Client,
    msg: tmi::Privmsg<'_>,
    tx: &mpsc::Sender<String>,
    args: Args,
) -> Result<()> {
    log::debug!("\nTwitch Message: {:?}", msg);
    log::info!(
        "Twitch Message from {}: {}",
        msg.sender().name(),
        msg.text()
    );

    if client.credentials().is_anon() {
        return Ok(());
    }

    let db_path = "db/twitch_chat.db";
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_history (
                id INTEGER PRIMARY KEY,
                user_id TEXT NOT NULL,
                message TEXT NOT NULL
            )",
        [],
    )?;

    let user_id = msg.sender().name();

    // Retrieve the chat history for the specific user
    let mut chat_messages: Vec<String> = conn
        .prepare("SELECT message FROM chat_history WHERE user_id = ?")?
        .query_map(params![user_id], |row| row.get(0))?
        .collect::<Result<_, _>>()?;

    // send message to the LLM and get an answer to send back to the user.
    // also send the message to the main LLM loop to keep history context of the conversation
    if !msg.text().starts_with("!help") && !msg.text().starts_with("!message") {
        // LLM Thread
        let (external_sender, mut external_receiver) = tokio::sync::mpsc::channel::<String>(100);
        let max_tokens = args.twitch_max_tokens_chat;
        let temperature = 0.8;
        let quantized = false;
        let max_messages = args.twitch_chat_history;

        let system_start_token = if args.twitch_model == "gemma" {
            "<start_of_turn>"
        } else {
            "<<SYS>>"
        };

        let system_end_token = if args.twitch_model == "gemma" {
            "<end_of_turn>"
        } else {
            "<</SYS>>"
        };

        let assistant_start_token = if args.twitch_model == "gemma" {
            "<start_of_turn>"
        } else {
            ""
        };

        let assistant_end_token = if args.twitch_model == "gemma" {
            "<end_of_turn>"
        } else {
            ""
        };

        let start_token = if args.twitch_model == "gemma" {
            "<start_of_turn>"
        } else {
            "[INST]"
        };

        let end_token = if args.twitch_model == "gemma" {
            "<end_of_turn>"
        } else {
            "[/INST]"
        };

        let bos_token = if args.twitch_model == "gemma" {
            ""
        } else {