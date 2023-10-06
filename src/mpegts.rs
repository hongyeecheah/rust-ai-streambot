
use crate::hexdump;
use crate::stream_data::StreamData;
use h264_reader::annexb::AnnexBReader;
use h264_reader::nal::{pps, sei, slice, sps, Nal, RefNal, UnitType};
use h264_reader::push::NalInterest;
use h264_reader::Context;
use hex_slice::AsHex;
use log::{debug, error, info};
use mpeg2ts_reader::demultiplex;
use mpeg2ts_reader::packet;
use mpeg2ts_reader::packet::Pid;
use mpeg2ts_reader::pes;
use mpeg2ts_reader::psi;
use mpeg2ts_reader::StreamType;
use scte35_reader;
use std::cell;
use std::cmp;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{self};
use tokio::task;
use tokio::time::Duration;

const DEBUG_PTS: bool = true;
const DEBUG_PAYLOAD: bool = false;
const DEBUG_PES: bool = true;
const DEBUG_PCR: bool = true;
const DEBUG_SCTE35: bool = true;

fn is_cea_608(itu_t_t35_data: &sei::user_data_registered_itu_t_t35::ItuTT35) -> bool {
    // In this example, we check if the ITU-T T.35 data matches the known format for CEA-608.
    // This is a simplified example and might need adjustment based on the actual data format.
    match itu_t_t35_data {
        sei::user_data_registered_itu_t_t35::ItuTT35::UnitedStates => true,
        _ => false,
    }
}

// This function checks if the byte is a standard ASCII character
fn is_standard_ascii(byte: u8) -> bool {
    byte >= 0x20 && byte <= 0x7F
}

// Function to check if the byte pair represents XDS data
fn is_xds(byte1: u8, byte2: u8) -> bool {
    // Implement logic to identify XDS data
    // Placeholder logic: Example only
    byte1 == 0x01 && byte2 >= 0x20 && byte2 <= 0x7F
}

// Function to decode CEA-608 CC1/CC2
fn decode_cea_608_cc1_cc2(byte1: u8, byte2: u8) -> Option<String> {
    decode_character(byte1, byte2)
    // The above line replaces the previous implementation and uses decode_character
    // to handle both ASCII and control codes.
}

fn decode_cea_608_xds(byte1: u8, byte2: u8) -> Option<String> {
    if is_xds(byte1, byte2) {
        Some(format!("XDS: {:02X} {:02X}", byte1, byte2))
    } else {
        None
    }
}

// Decode CEA-608 characters, including control codes
fn decode_character(byte1: u8, byte2: u8) -> Option<String> {
    debug!("Decoding: {:02X} {:02X}", byte1, byte2); // Debugging

    // Handle standard ASCII characters
    if is_standard_ascii(byte1) && is_standard_ascii(byte2) {
        return Some(format!("{}{}", byte1 as char, byte2 as char));
    }

    // Handle special control characters (Example)
    // This is a simplified version, actual implementation may vary based on control characters
    match (byte1, byte2) {
        (0x14, 0x2C) => Some(String::from("[Clear Caption]")),
        (0x14, 0x20) => Some(String::from("[Roll-Up Caption]")),
        // Add more control character handling here
        _ => {
            error!("Unhandled control character: {:02X} {:02X}", byte1, byte2); // Debugging
            None
        }
    }
}

// Simplified CEA-608 decoding function
// Main CEA-608 decoding function
fn decode_cea_608(data: &[u8]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut captions_cc1 = Vec::new();
    let mut captions_cc2 = Vec::new();
    let mut xds_data = Vec::new();

    for chunk in data.chunks(3) {
        if chunk.len() == 3 {
            match chunk[0] {
                0x04 => {
                    if let Some(decoded) = decode_cea_608_cc1_cc2(chunk[1], chunk[2]) {
                        captions_cc1.push(decoded);
                    } else if let Some(decoded) = decode_cea_608_xds(chunk[1], chunk[2]) {
                        xds_data.push(decoded);
                    }
                }
                0x05 => {
                    if let Some(decoded) = decode_cea_608_cc1_cc2(chunk[1], chunk[2]) {
                        captions_cc2.push(decoded);
                    }
                }
                _ => debug!("Unknown caption channel: {:02X}", chunk[0]),
            }
        }
    }

    (captions_cc1, captions_cc2, xds_data)
}

pub struct DumpSpliceInfoProcessor {
    pub elementary_pid: Option<Pid>,
    pub last_pcr: Rc<cell::Cell<Option<packet::ClockRef>>>,
}
impl scte35_reader::SpliceInfoProcessor for DumpSpliceInfoProcessor {
    fn process(
        &self,
        header: scte35_reader::SpliceInfoHeader<'_>,
        command: scte35_reader::SpliceCommand,
        descriptors: scte35_reader::SpliceDescriptors<'_>,
    ) {
        if DEBUG_SCTE35 {
            if let Some(elementary_pid) = self.elementary_pid {
                print!("{:?} ", elementary_pid);
            }
            if let Some(pcr) = self.last_pcr.as_ref().get() {
                print!("Last {:?}: ", pcr)
            }
            print!("{:?} {:#?}", header, command);
        }
        if let scte35_reader::SpliceCommand::SpliceInsert { splice_detail, .. } = command {
            if let scte35_reader::SpliceInsert::Insert { splice_mode, .. } = splice_detail {
                if let scte35_reader::SpliceMode::Program(scte35_reader::SpliceTime::Timed(t)) =
                    splice_mode
                {
                    if let Some(time) = t {
                        let time_ref = mpeg2ts_reader::packet::ClockRef::from_parts(time, 0);
                        if let Some(pcr) = self.last_pcr.as_ref().get() {
                            let mut diff = time_ref.base() as i64 - pcr.base() as i64;
                            if diff < 0 {
                                diff += (std::u64::MAX / 2) as i64;
                            }
                            if DEBUG_SCTE35 {
                                print!(" {}ms after last PCR", diff / 90);
                            }
                        }
                    }
                }
            }
        }
        if DEBUG_SCTE35 {
            println!();
        }
        for d in &descriptors {
            if DEBUG_SCTE35 {
                println!(" - {:#?}", d);
            }
        }
    }
}

pub struct Scte35StreamConsumer {
    section: psi::SectionPacketConsumer<
        psi::CompactSyntaxSectionProcessor<
            psi::BufferCompactSyntaxParser<
                scte35_reader::Scte35SectionProcessor<DumpSpliceInfoProcessor, DumpDemuxContext>,
            >,
        >,
    >,
}

impl Scte35StreamConsumer {
    fn new(elementary_pid: Pid, last_pcr: Rc<cell::Cell<Option<packet::ClockRef>>>) -> Self {
        let parser = scte35_reader::Scte35SectionProcessor::new(DumpSpliceInfoProcessor {
            elementary_pid: Some(elementary_pid),
            last_pcr,
        });
        Scte35StreamConsumer {
            section: psi::SectionPacketConsumer::new(psi::CompactSyntaxSectionProcessor::new(
                psi::BufferCompactSyntaxParser::new(parser),
            )),
        }
    }

    fn construct(
        last_pcr: Rc<cell::Cell<Option<packet::ClockRef>>>,
        program_pid: packet::Pid,
        pmt: &psi::pmt::PmtSection<'_>,
        stream_info: &psi::pmt::StreamInfo<'_>,
    ) -> DumpFilterSwitch {
        if scte35_reader::is_scte35(pmt) {
            if DEBUG_SCTE35 {
                info!(
                    "Program {:?}: {:?} has type {:?}, but PMT has 'CUEI' registration_descriptor that would indicate SCTE-35 content",
                    program_pid,
                    stream_info.elementary_pid(),
                    stream_info.stream_type()
                );
            }
            DumpFilterSwitch::Scte35(Scte35StreamConsumer::new(
                stream_info.elementary_pid(),
                last_pcr,
            ))
        } else {
            if DEBUG_SCTE35 {
                info!(
                    "Program {:?}: {:?} has type {:?}, but PMT lacks 'CUEI' registration_descriptor that would indicate SCTE-35 content",
                    program_pid,
                    stream_info.elementary_pid(),