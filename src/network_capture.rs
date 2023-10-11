
/*
 * network_capture.rs
 * ------------------
 * Author: Chris Kennedy February @2024
 *
 * This file contains the network capture module for RsLLM.
*/

#[cfg(feature = "dpdk_enabled")]
use capsule::config::{load_config, DPDKConfig};
#[cfg(feature = "dpdk_enabled")]
use capsule::dpdk;
#[cfg(all(feature = "dpdk_enabled", target_os = "linux"))]
use capsule::prelude::*;
use futures::stream::StreamExt;
use log::{debug, error, info};
use pcap::{Active, Capture, Device, PacketCodec};
use std::error::Error as StdError;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{self};
use tokio::task::JoinHandle;
use tokio::time::Instant;

// Define your custom PacketCodec
pub struct BoxCodec;

impl PacketCodec for BoxCodec {
    type Item = Box<[u8]>;

    fn decode(&mut self, packet: pcap::Packet) -> Self::Item {
        packet.data.into()
    }
}

// Define a custom error for when the target device is not found
#[derive(Debug)]
struct DeviceNotFoundError;

impl std::error::Error for DeviceNotFoundError {}

impl DeviceNotFoundError {
    #[allow(dead_code)]
    fn new() -> ErrorWrapper {
        ErrorWrapper(Box::new(Self))
    }
}

impl fmt::Display for DeviceNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Target device not found")
    }
}

struct ErrorWrapper(Box<dyn StdError + Send + Sync>);

impl fmt::Debug for ErrorWrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {