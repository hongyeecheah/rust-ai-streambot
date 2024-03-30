
/*
 * stream_data.rs
 *
 * Data structure for the stream data
*/

use crate::current_unix_timestamp_ms;
use ahash::AHashMap;
use lazy_static::lazy_static;
use log::{debug, error, info};
use rtp::RtpReader;
use rtp_rs as rtp;
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc, sync::Mutex};

// global variable to store the MpegTS PID Map (initially empty)
lazy_static! {
    static ref PID_MAP: Mutex<AHashMap<u16, Arc<StreamData>>> = Mutex::new(AHashMap::new());
}

pub fn get_pid_map() -> String {
    let pid_map = PID_MAP.lock().unwrap();
    let mut result = String::new();

    for (pid, stream_data_arc) in pid_map.iter() {
        let stream_data = Arc::clone(stream_data_arc);
        // Assuming you have implemented Display or a similar method to summarize StreamData
        // Or manually concatenate stream data fields here
        let stream_data_summary = format!(
            "PID: {}, PMT PID: {}, Program Number: {}, Stream Type: {}, Continuity Counter: {}, Timestamp: {}, Bitrate: {}, Bitrate Max: {}, Bitrate Min: {}, Bitrate Avg: {}, IAT: {}, IAT Max: {}, IAT Min: {}, IAT Avg: {}, Error Count: {}, Last Arrival Time: {}, Start Time: {}, Total Bits: {}, Count: {}, RTP Timestamp: {}, RTP Payload Type: {}, RTP Payload Type Name: {}, RTP Line Number: {}, RTP Line Offset: {}, RTP Line Length: {}, RTP Field ID: {}, RTP Line Continuation: {}, RTP Extended Sequence Number: {}",
            pid,
            stream_data.pmt_pid,
            stream_data.program_number,
            stream_data.stream_type,
            stream_data.continuity_counter,
            stream_data.timestamp,
            stream_data.bitrate,
            stream_data.bitrate_max,
            stream_data.bitrate_min,
            stream_data.bitrate_avg,
            stream_data.iat,
            stream_data.iat_max,
            stream_data.iat_min,
            stream_data.iat_avg,
            stream_data.error_count,
            stream_data.last_arrival_time,
            stream_data.start_time,
            stream_data.total_bits,
            stream_data.count,
            stream_data.rtp_timestamp,
            stream_data.rtp_payload_type,
            stream_data.rtp_payload_type_name,
            stream_data.rtp_line_number,
            stream_data.rtp_line_offset,
            stream_data.rtp_line_length,
            stream_data.rtp_field_id,
            stream_data.rtp_line_continuation,
            stream_data.rtp_extended_sequence_number
        );
        result.push_str(&format!("{}\n", stream_data_summary));
    }

    result
}

// constant for PAT PID
pub const PAT_PID: u16 = 0;
pub const TS_PACKET_SIZE: usize = 188;

pub struct PatEntry {
    pub program_number: u16,
    pub pmt_pid: u16,
}

pub struct PmtEntry {
    pub stream_pid: u16,
    pub stream_type: u8, // Stream type (e.g., 0x02 for MPEG video)
}

pub struct Pmt {
    pub entries: Vec<PmtEntry>,
}

#[derive(Clone, PartialEq)]
pub enum Codec {
    NONE,
    MPEG2,
    H264,
    H265,
}

impl fmt::Display for Codec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Codec::NONE => write!(f, "NONE"),
            Codec::MPEG2 => write!(f, "MPEG2"),
            Codec::H264 => write!(f, "H264"),
            Codec::H265 => write!(f, "H265"),
        }
    }
}

// StreamData struct
#[derive(Serialize, Deserialize, Debug)]
pub struct StreamData {
    pub pid: u16,
    pub pmt_pid: u16,
    pub program_number: u16,
    pub stream_type: String, // "video", "audio", "text"
    pub continuity_counter: u8,
    pub timestamp: u64,
    pub bitrate: u32,
    pub bitrate_max: u32,
    pub bitrate_min: u32,
    pub bitrate_avg: u32,
    pub iat: u64,
    pub iat_max: u64,
    pub iat_min: u64,
    pub iat_avg: u64,
    pub error_count: u32,
    pub last_arrival_time: u64,
    pub start_time: u64, // field for start time
    pub total_bits: u64, // field for total bits
    pub count: u32,      // field for count
    #[serde(skip)]
    pub packet: Arc<Vec<u8>>, // The actual MPEG-TS packet data
    pub packet_start: usize, // Offset into the data
    pub packet_len: usize, // Offset into the data
    // SMPTE 2110 fields
    pub rtp_timestamp: u32,
    pub rtp_payload_type: u8,
    pub rtp_payload_type_name: String,
    pub rtp_line_number: u16,
    pub rtp_line_offset: u16,
    pub rtp_line_length: u16,
    pub rtp_field_id: u8,
    pub rtp_line_continuation: u8,
    pub rtp_extended_sequence_number: u16,
}

impl Clone for StreamData {
    fn clone(&self) -> Self {
        StreamData {