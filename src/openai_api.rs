/*
Implement the OpenAI API generically for any LLM following it
Chris Kennedy @2024 MIT license
*/

use bytes::Bytes;
use chrono::{TimeZone, Utc};
use log::{debug, error, info};
use reqwest::Client;
