
/// Module `tts` - Handles Text-to-Speech (TTS) conversion using an API service.
/// Credit: https://github.com/Shahrooze/simple-open-ai/tree/main original source
///
/// # Overview
/// This module provides functionality to convert text input to speech audio,
/// interfacing with an external TTS service API.
///
/// # Dependencies
/// - `reqwest`: A high-level HTTP client for making requests.
/// - `bytes`: Utilities for working with bytes.
/// - `serde`: Serialization and deserialization framework, used here to serialize request data.
///
/// # Constants
/// `ENDPOINT`: The API endpoint for the TTS service.
///
/// # Structures
/// `Request`: Represents a TTS API request with parameters for the speech model, input text, voice settings,
/// and desired response format.
///
/// # Enums
/// `ResponseFormat`: Enumerates the possible audio formats for the TTS response, including MP3, Opus, AAC, and FLAC.
///
/// # Error Handling
/// Utilizes `ApiError` for consistent error management across the application.
///
use bytes::Bytes;
use reqwest::Client;
use serde::Serialize;
const ENDPOINT: &str = "https://api.openai.com/v1/audio/speech";
use crate::ApiError;
use log::debug;

impl std::fmt::Display for Voice {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Voice::Alloy => write!(f, "alloy"),
            Voice::Echo => write!(f, "echo"),
            Voice::Fable => write!(f, "fable"),
            Voice::Onyx => write!(f, "onyx"),
            Voice::Nova => write!(f, "nova"),
            Voice::Shimmer => write!(f, "shimmer"),
        }
    }