
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
            pid: self.pid,
            pmt_pid: self.pmt_pid,
            program_number: self.program_number,
            stream_type: self.stream_type.clone(),
            continuity_counter: self.continuity_counter,
            timestamp: self.timestamp,
            bitrate: self.bitrate,
            bitrate_max: self.bitrate_max,
            bitrate_min: self.bitrate_min,
            bitrate_avg: self.bitrate_avg,
            iat: self.iat,
            iat_max: self.iat_max,
            iat_min: self.iat_min,
            iat_avg: self.iat_avg,
            error_count: self.error_count,
            last_arrival_time: self.last_arrival_time,
            start_time: self.start_time,
            total_bits: self.total_bits,
            count: self.count,
            packet: Arc::new(Vec::new()), // Initialize as empty with Arc
            packet_start: 0,
            packet_len: 0,
            rtp_timestamp: self.rtp_timestamp,
            rtp_payload_type: self.rtp_payload_type,
            rtp_payload_type_name: self.rtp_payload_type_name.clone(),
            rtp_line_number: self.rtp_line_number,
            rtp_line_offset: self.rtp_line_offset,
            rtp_line_length: self.rtp_line_length,
            rtp_field_id: self.rtp_field_id,
            rtp_line_continuation: self.rtp_line_continuation,
            rtp_extended_sequence_number: self.rtp_extended_sequence_number,
        }
    }
}

// StreamData implementation
impl StreamData {
    pub fn new(
        packet: Arc<Vec<u8>>,
        packet_start: usize,
        packet_len: usize,
        pid: u16,
        stream_type: String,
        start_time: u64,
        timestamp: u64,
        continuity_counter: u8,
    ) -> Self {
        let last_arrival_time = current_unix_timestamp_ms().unwrap_or(0);
        StreamData {
            pid,
            pmt_pid: 0xFFFF,
            program_number: 0,
            stream_type,
            continuity_counter,
            timestamp,
            bitrate: 0,
            bitrate_max: 0,
            bitrate_min: 0,
            bitrate_avg: 0,
            iat: 0,
            iat_max: 0,
            iat_min: 0,
            iat_avg: 0,
            error_count: 0,
            last_arrival_time,
            start_time,    // Initialize start time
            total_bits: 0, // Initialize total bits
            count: 0,      // Initialize count
            packet: packet,
            packet_start: packet_start,
            packet_len: packet_len,
            // SMPTE 2110 fields
            rtp_timestamp: 0,
            rtp_payload_type: 0,
            rtp_payload_type_name: "".to_string(),
            rtp_line_number: 0,
            rtp_line_offset: 0,
            rtp_line_length: 0,
            rtp_field_id: 0,
            rtp_line_continuation: 0,
            rtp_extended_sequence_number: 0,
        }
    }
    // set RTP fields
    pub fn set_rtp_fields(
        &mut self,
        rtp_timestamp: u32,
        rtp_payload_type: u8,
        rtp_payload_type_name: String,
        rtp_line_number: u16,
        rtp_line_offset: u16,
        rtp_line_length: u16,
        rtp_field_id: u8,
        rtp_line_continuation: u8,
        rtp_extended_sequence_number: u16,
    ) {
        self.rtp_timestamp = rtp_timestamp;
        self.rtp_payload_type = rtp_payload_type;
        self.rtp_payload_type_name = rtp_payload_type_name;
        self.rtp_line_number = rtp_line_number;
        self.rtp_line_offset = rtp_line_offset;
        self.rtp_line_length = rtp_line_length;
        self.rtp_field_id = rtp_field_id;
        self.rtp_line_continuation = rtp_line_continuation;
        self.rtp_extended_sequence_number = rtp_extended_sequence_number;
    }
    pub fn update_stream_type(&mut self, stream_type: String) {
        self.stream_type = stream_type;
    }
    pub fn increment_error_count(&mut self, error_count: u32) {
        self.error_count += error_count;
    }
    pub fn increment_count(&mut self, count: u32) {
        self.count += count;
    }
    pub fn set_continuity_counter(&mut self, continuity_counter: u8) {
        // check for continuity continuous increment and wrap around from 0 to 15
        let previous_continuity_counter = self.continuity_counter;
        self.continuity_counter = continuity_counter & 0x0F;
        // check if we incremented without loss
        if self.continuity_counter != previous_continuity_counter + 1
            && self.continuity_counter != previous_continuity_counter
        {
            // check if we wrapped around from 15 to 0
            if self.continuity_counter == 0 {
                // check if previous value was 15
                if previous_continuity_counter == 15 {
                    // no loss
                    return;
                }
            }
            // loss
            self.increment_error_count(1);
            error!(
                "Continuity Counter Error: PID: {} Previous: {} Current: {}",
                self.pid, previous_continuity_counter, self.continuity_counter
            );
        }
        self.continuity_counter = continuity_counter;
    }
    pub fn update_stats(&mut self, packet_size: usize, arrival_time: u64) {
        let bits = packet_size as u64 * 8; // Convert bytes to bits

        // Elapsed time in milliseconds
        let elapsed_time_ms = arrival_time.checked_sub(self.start_time).unwrap_or(0);

        if elapsed_time_ms > 0 {
            let elapsed_time_sec = elapsed_time_ms as f64 / 1000.0;
            self.bitrate = (self.total_bits as f64 / elapsed_time_sec) as u32;

            // Bitrate max
            if self.bitrate > self.bitrate_max {
                self.bitrate_max = self.bitrate;
            }

            // Bitrate min
            if self.bitrate < self.bitrate_min {
                self.bitrate_min = self.bitrate;
            }

            // Bitrate avg
            self.bitrate_avg = (self.bitrate_avg + self.bitrate) / 2;
        }

        self.total_bits += bits; // Accumulate total bits

        // IAT calculation remains the same
        let iat = arrival_time
            .checked_sub(self.last_arrival_time)
            .unwrap_or(0);
        self.iat = iat;

        // IAT max
        if iat > self.iat_max {
            self.iat_max = iat;
        }

        // IAT min
        if iat < self.iat_min {
            self.iat_min = iat;
        }

        // IAT avg
        self.iat_avg = (self.iat_avg + iat) / 2;

        self.last_arrival_time = arrival_time;
    }
}

pub struct Tr101290Errors {
    // p1 errors
    pub ts_sync_byte_errors: u32,
    pub sync_byte_errors: u32,
    pub continuity_counter_errors: u32,
    pub pat_errors: u32,
    pub pmt_errors: u32,
    pub pid_map_errors: u32,
    // p2 errors
    pub transport_error_indicator_errors: u32,
    pub crc_errors: u32,
    pub pcr_repetition_errors: u32,
    pub pcr_discontinuity_indicator_errors: u32,
    pub pcr_accuracy_errors: u32,
    pub pts_errors: u32,
    pub cat_errors: u32,
}

impl fmt::Display for Tr101290Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\
            TS Sync Byte Errors: {}, \
            Sync Byte Errors: {}, \
            Continuity Counter Errors: {}, \
            PAT Errors: {}, \
            PMT Errors: {}, \
            PID Map Errors: {}, \
            Transport Error Indicator Errors: {}, \
            CRC Errors: {}, \
            PCR Repetition Errors: {}, \
            PCR Discontinuity Indicator Errors: {}, \
            PCR Accuracy Errors: {}, \
            PTS Errors: {}, \
            CAT Errors: {}",
            self.ts_sync_byte_errors,
            self.sync_byte_errors,
            self.continuity_counter_errors,
            self.pat_errors,
            self.pmt_errors,
            self.pid_map_errors,
            // p2 errors
            self.transport_error_indicator_errors,
            self.crc_errors,
            self.pcr_repetition_errors,
            self.pcr_discontinuity_indicator_errors,
            self.pcr_accuracy_errors,
            self.pts_errors,
            self.cat_errors
        )
    }
}

impl Tr101290Errors {
    pub fn new() -> Self {
        Tr101290Errors {
            ts_sync_byte_errors: 0,
            sync_byte_errors: 0,
            continuity_counter_errors: 0,
            pat_errors: 0,
            pmt_errors: 0,
            pid_map_errors: 0,
            // p2
            transport_error_indicator_errors: 0,
            crc_errors: 0,
            pcr_repetition_errors: 0,
            pcr_discontinuity_indicator_errors: 0,
            pcr_accuracy_errors: 0,
            pts_errors: 0,
            cat_errors: 0,
        }
    }
}

// TR 101 290 Priority 1 Check
pub fn tr101290_p1_check(packet: &[u8], errors: &mut Tr101290Errors) {
    // p1
    if packet[0] != 0x47 {
        errors.sync_byte_errors += 1;
    }

    // TODO: ... other checks, updating the respective counters ...
}

// TR 101 290 Priority 2 Check
pub fn tr101290_p2_check(packet: &[u8], errors: &mut Tr101290Errors) {
    // p2

    if (packet[1] & 0x80) != 0 {
        errors.transport_error_indicator_errors += 1;
    }
    // TODO: ... other checks, updating the respective counters ...
}

// Implement a function to extract PID from a packet
pub fn extract_pid(packet: &[u8]) -> u16 {
    if packet.len() < TS_PACKET_SIZE {
        return 0; // Packet size is incorrect
    }

    let transport_error = (packet[1] & 0x80) != 0;
    if transport_error {
        return 0xFFFF; // Packet has a transport error
    }

    // Extract PID from packet
    ((packet[1] as u16 & 0x1F) << 8) | packet[2] as u16
}

// Story the PAT packet and PMT PID
pub struct PmtInfo {
    pub pid: u16,
    pub packet: Vec<u8>,
}

// Helper function to parse PAT and update global PAT packet storage
pub fn parse_and_store_pat(packet: &[u8]) -> PmtInfo {
    let pat_entries = parse_pat(packet);
    let mut pmt_info = PmtInfo {
        pid: 0xFFFF,
        packet: Vec::new(),
    };
    pmt_info.packet = packet.to_vec();

    // Assuming there's only one program for simplicity, update PMT PID
    if let Some(first_entry) = pat_entries.first() {
        pmt_info.pid = first_entry.pmt_pid;
    }
    pmt_info
}

pub fn parse_pat(packet: &[u8]) -> Vec<PatEntry> {
    let mut entries = Vec::new();

    // Check for minimum packet size
    if packet.len() < TS_PACKET_SIZE {
        return entries;
    }

    // Check if Payload Unit Start Indicator (PUSI) is set
    let pusi = (packet[1] & 0x40) != 0;
    if !pusi {
        // If Payload Unit Start Indicator is not set, this packet does not start a new PAT
        return entries;
    }

    let adaptation_field_control = (packet[3] & 0x30) >> 4;
    let mut offset = 4; // start after TS header

    // Check for adaptation field and skip it
    if adaptation_field_control == 0x02 || adaptation_field_control == 0x03 {
        let adaptation_field_length = packet[4] as usize;
        offset += 1 + adaptation_field_length; // +1 for the length byte itself
    }

    // Pointer field indicates the start of the PAT section
    let pointer_field = packet[offset] as usize;
    offset += 1 + pointer_field; // Skip pointer field

    // Now, 'offset' points to the start of the PAT section
    while offset + 4 <= packet.len() {
        let program_number = ((packet[offset] as u16) << 8) | (packet[offset + 1] as u16);
        let pmt_pid = (((packet[offset + 2] as u16) & 0x1F) << 8) | (packet[offset + 3] as u16);

        // Only add valid entries (non-zero program_number and pmt_pid)
        if program_number != 0 && pmt_pid != 0 && pmt_pid < 0x1FFF && program_number < 100 {
            entries.push(PatEntry {
                program_number,
                pmt_pid,
            });
        }

        debug!(
            "ParsePAT: Program Number: {} PMT PID: {}",
            program_number, pmt_pid
        );

        offset += 4; // Move to the next PAT entry
    }

    entries
}

pub fn parse_pmt(packet: &[u8]) -> Pmt {
    let mut entries = Vec::new();
    let program_number = ((packet[8] as u16) << 8) | (packet[9] as u16);

    // Calculate the starting position for stream entries
    let section_length = (((packet[6] as usize) & 0x0F) << 8) | packet[7] as usize;
    let program_info_length = (((packet[15] as usize) & 0x0F) << 8) | packet[16] as usize;
    let mut i = 17 + program_info_length; // Starting index of the first stream in the PMT

    debug!(
        "ParsePMT: Program Number: {} PMT PID: {} starting at position {}",
        program_number,
        extract_pid(packet),
        i
    );
    while i + 5 <= packet.len() && i < 17 + section_length - 4 {
        let stream_type = packet[i];
        let stream_pid = (((packet[i + 1] as u16) & 0x1F) << 8) | (packet[i + 2] as u16);
        let es_info_length = (((packet[i + 3] as usize) & 0x0F) << 8) | packet[i + 4] as usize;
        i += 5 + es_info_length; // Update index to point to next stream's info

        entries.push(PmtEntry {
            stream_pid,
            stream_type,
        });
        debug!(
            "ParsePMT: Stream PID: {}, Stream Type: {}",
            stream_pid, stream_type
        );
    }

    Pmt { entries }
}

// Invoke this function for each MPEG-TS packet
pub fn process_packet(
    stream_data_packet: &mut StreamData,
    errors: &mut Tr101290Errors,
    is_mpegts: bool,
    pmt_pid: u16,
) {
    let packet: &[u8] = &stream_data_packet.packet[stream_data_packet.packet_start
        ..stream_data_packet.packet_start + stream_data_packet.packet_len];
    tr101290_p1_check(packet, errors);
    tr101290_p2_check(packet, errors);

    let pid = stream_data_packet.pid;
    let arrival_time = current_unix_timestamp_ms().unwrap_or(0);

    let mut pid_map = PID_MAP.lock().unwrap();

    // TODO: high debug level output, may need a flag specific to this dump
    //info!("PID Map Contents: {:#?}", pid_map);

    // Check if the PID map already has an entry for this PID
    match pid_map.get_mut(&pid) {
        Some(stream_data_arc) => {
            // Existing StreamData instance found, update it
            let mut stream_data = Arc::clone(stream_data_arc);
            Arc::make_mut(&mut stream_data).update_stats(packet.len(), arrival_time);
            Arc::make_mut(&mut stream_data).increment_count(1);
            if stream_data.pid != 0x1FFF && is_mpegts {
                Arc::make_mut(&mut stream_data)
                    .set_continuity_counter(stream_data_packet.continuity_counter);
            }
            let uptime = arrival_time - stream_data.start_time;

            // print out each field of structure
            debug!("STATUS::PACKET:MODIFY[{}] pid: {} stream_type: {} bitrate: {} bitrate_max: {} bitrate_min: {} bitrate_avg: {} iat: {} iat_max: {} iat_min: {} iat_avg: {} errors: {} continuity_counter: {} timestamp: {} uptime: {} packet_offset: {}, packet_len: {}",
                stream_data.pid, stream_data.pid, stream_data.stream_type, stream_data.bitrate, stream_data.bitrate_max, stream_data.bitrate_min, stream_data.bitrate_avg, stream_data.iat, stream_data.iat_max, stream_data.iat_min, stream_data.iat_avg, stream_data.error_count, stream_data.continuity_counter, stream_data.timestamp, uptime, stream_data_packet.packet_start, stream_data_packet.packet_len);

            stream_data_packet.bitrate = stream_data.bitrate;
            stream_data_packet.bitrate_avg = stream_data.bitrate_avg;
            stream_data_packet.bitrate_max = stream_data.bitrate_max;
            stream_data_packet.bitrate_min = stream_data.bitrate_min;
            stream_data_packet.iat = stream_data.iat;
            stream_data_packet.iat_avg = stream_data.iat_avg;
            stream_data_packet.iat_max = stream_data.iat_max;
            stream_data_packet.iat_min = stream_data.iat_min;
            stream_data_packet.stream_type = stream_data.stream_type.clone();
            stream_data_packet.start_time = stream_data.start_time;
            stream_data_packet.error_count = stream_data.error_count;
            stream_data_packet.last_arrival_time = stream_data.last_arrival_time;
            stream_data_packet.total_bits = stream_data.total_bits;
            stream_data_packet.count = stream_data.count;

            // write the stream_data back to the pid_map with modified values
            pid_map.insert(pid, stream_data);
        }
        None => {
            // No StreamData instance found for this PID, possibly no PMT yet
            if pmt_pid != 0xFFFF {
                debug!("ProcessPacket: New PID {} Found, adding to PID map.", pid);