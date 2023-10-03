
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
