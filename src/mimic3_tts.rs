
use crate::ApiError; // Assuming ApiError is defined in lib.rs and is in scope
use bytes::Bytes;
use log::debug;
use reqwest::Client;
use serde::Serialize;

const ENDPOINT: &str = "http://localhost:59125/api/tts"; // Mimic3 endpoint
