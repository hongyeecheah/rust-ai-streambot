
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
                    stream_info.stream_type()
                );
            }
            DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
        }
    }
}
impl demultiplex::PacketFilter for Scte35StreamConsumer {
    type Ctx = DumpDemuxContext;
    fn consume(&mut self, ctx: &mut Self::Ctx, pk: &packet::Packet<'_>) {
        self.section.consume(ctx, pk);
    }
}

pub struct PcrWatch(Rc<cell::Cell<Option<packet::ClockRef>>>);
impl demultiplex::PacketFilter for PcrWatch {
    type Ctx = DumpDemuxContext;
    fn consume(&mut self, _ctx: &mut Self::Ctx, pk: &packet::Packet<'_>) {
        if let Some(af) = pk.adaptation_field() {
            if let Ok(pcr) = af.pcr() {
                self.0.set(Some(pcr));
                if DEBUG_PCR {
                    info!("Got PCR: {:?}", pcr);
                }
            }
        }
    }
}

mpeg2ts_reader::packet_filter_switch! {
    DumpFilterSwitch<DumpDemuxContext> {
        Pat: demultiplex::PatPacketFilter<DumpDemuxContext>,
        Pes: pes::PesPacketFilter<DumpDemuxContext,PtsDumpElementaryStreamConsumer>,
        Pmt: demultiplex::PmtPacketFilter<DumpDemuxContext>,
        Null: demultiplex::NullPacketFilter<DumpDemuxContext>,
        Scte35: Scte35StreamConsumer,
        Pcr: PcrWatch,
    }
}
pub struct DumpDemuxContext {
    changeset: demultiplex::FilterChangeset<DumpFilterSwitch>,
    last_pcrs: HashMap<packet::Pid, Rc<cell::Cell<Option<packet::ClockRef>>>>,
}
impl DumpDemuxContext {
    pub fn new() -> Self {
        DumpDemuxContext {
            changeset: demultiplex::FilterChangeset::default(),
            last_pcrs: HashMap::new(),
        }
    }
    pub fn last_pcr(&self, program_pid: packet::Pid) -> Rc<cell::Cell<Option<packet::ClockRef>>> {
        self.last_pcrs
            .get(&program_pid)
            .expect("last_pcrs entry didn't exist on call to last_pcr()")
            .clone()
    }
}
impl demultiplex::DemuxContext for DumpDemuxContext {
    type F = DumpFilterSwitch;

    fn filter_changeset(&mut self) -> &mut demultiplex::FilterChangeset<Self::F> {
        &mut self.changeset
    }

    fn construct(&mut self, req: demultiplex::FilterRequest<'_, '_>) -> Self::F {
        match req {
            demultiplex::FilterRequest::ByPid(packet::Pid::PAT) => {
                DumpFilterSwitch::Pat(demultiplex::PatPacketFilter::default())
            }
            // 'Stuffing' data on PID 0x1fff may be used to pad-out parts of the transport stream
            // so that it has constant overall bitrate.  This causes it to be ignored if present.
            demultiplex::FilterRequest::ByPid(mpeg2ts_reader::STUFFING_PID) => {
                DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
            }
            // This match-arm installs our application-specific handling for each H264 stream
            // discovered within the transport stream,
            demultiplex::FilterRequest::ByStream {
                stream_type: StreamType::H264,
                pmt,
                stream_info,
                ..
            } => PtsDumpElementaryStreamConsumer::construct(pmt, stream_info),
            demultiplex::FilterRequest::ByStream {
                program_pid,
                stream_type: scte35_reader::SCTE35_STREAM_TYPE,
                pmt,
                stream_info,
            } => Scte35StreamConsumer::construct(
                self.last_pcr(program_pid),
                program_pid,
                pmt,
                stream_info,
            ),
            demultiplex::FilterRequest::ByStream { program_pid, .. } => {
                DumpFilterSwitch::Pcr(PcrWatch(self.last_pcr(program_pid)))
            }
            demultiplex::FilterRequest::Pmt {
                pid,
                program_number,
            } => {
                // prepare structure needed to print PCR values later on
                self.last_pcrs.insert(pid, Rc::new(cell::Cell::new(None)));
                DumpFilterSwitch::Pmt(demultiplex::PmtPacketFilter::new(pid, program_number))
            }
            demultiplex::FilterRequest::Nit { .. } => {
                DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
            }
            demultiplex::FilterRequest::ByPid(_) => {
                DumpFilterSwitch::Null(demultiplex::NullPacketFilter::default())
            }
        }
    }
}

// Implement the ElementaryStreamConsumer to just dump and PTS/DTS timestamps to stdout
pub struct PtsDumpElementaryStreamConsumer {
    pid: packet::Pid,
    len: Option<usize>,
}
impl PtsDumpElementaryStreamConsumer {
    fn construct(
        _pmt_sect: &psi::pmt::PmtSection,
        stream_info: &psi::pmt::StreamInfo,
    ) -> DumpFilterSwitch {
        let filter = pes::PesPacketFilter::new(PtsDumpElementaryStreamConsumer {
            pid: stream_info.elementary_pid(),
            len: None,
        });
        DumpFilterSwitch::Pes(filter)
    }
}
impl pes::ElementaryStreamConsumer<DumpDemuxContext> for PtsDumpElementaryStreamConsumer {
    fn start_stream(&mut self, _ctx: &mut DumpDemuxContext) {}
    fn begin_packet(&mut self, _ctx: &mut DumpDemuxContext, header: pes::PesHeader) {
        match header.contents() {
            pes::PesContents::Parsed(Some(parsed)) => {
                if DEBUG_PTS {
                    match parsed.pts_dts() {
                        Ok(pes::PtsDts::PtsOnly(Ok(pts))) => {
                            print!("{:?}: pts {:#08x}                ", self.pid, pts.value())
                        }
                        Ok(pes::PtsDts::Both {
                            pts: Ok(pts),
                            dts: Ok(dts),
                        }) => print!(
                            "{:?}: pts {:#08x} dts {:#08x} ",
                            self.pid,
                            pts.value(),
                            dts.value()
                        ),
                        _ => (),
                    }
                }
                let payload = parsed.payload();