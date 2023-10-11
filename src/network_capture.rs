
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
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for ErrorWrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for ErrorWrapper {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0.source()
    }
}

trait Packet: Send {
    fn data(&self) -> &[u8];
}

// Common interface for DPDK functionality
trait DpdkPort: Send {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn stop(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn rx_burst(&self, packets: &mut Vec<Box<dyn Packet>>) -> Result<(), anyhow::Error>;
    // Other necessary methods...
}

// Implementation for Linux with DPDK enabled
#[cfg(all(feature = "dpdk_enabled", target_os = "linux"))]
struct RealDpdkPort(dpdk::Port);

#[cfg(all(feature = "dpdk_enabled", target_os = "linux"))]
impl DpdkPort for RealDpdkPort {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.0.start()?;
        Ok(())
    }

    fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.0.stop()?;
        Ok(())
    }
    fn rx_burst(&self, packets: &mut Vec<Box<dyn Packet>>) -> Result<(), anyhow::Error> {
        // Logic for rx_burst...
        Ok(())
    }
}

#[cfg(all(feature = "dpdk_enabled", target_os = "linux"))]
fn init_dpdk(
    port_id: u16,
    promiscuous_mode: bool,
) -> Result<Box<dyn DpdkPort>, Box<dyn std::error::Error>> {
    // Initialize capsule environment
    let config = load_config()?;
    dpdk::eal::init(config)?;

    // Configure network interface
    let port = dpdk::Port::new(port_id)?;
    port.configure()?;

    // Set promiscuous mode if needed
    if promiscuous_mode {
        port.set_promiscuous(true)?;
    }

    // Start the port
    port.start()?;

    Ok(Box::new(RealDpdkPort(port)))
}

// Placeholder implementation for non-Linux or DPDK disabled builds
#[cfg(not(all(feature = "dpdk_enabled", target_os = "linux")))]
struct DummyDpdkPort;

#[cfg(not(all(feature = "dpdk_enabled", target_os = "linux")))]
impl DpdkPort for DummyDpdkPort {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        Err("DPDK is not supported on this OS".into())
    }

    fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        Err("DPDK is not supported on this OS".into())
    }

    fn rx_burst(&self, _packets: &mut Vec<Box<dyn Packet>>) -> Result<(), anyhow::Error> {
        Err(anyhow::Error::msg("DPDK is not supported on this OS"))
    }
}

#[cfg(not(all(feature = "dpdk_enabled", target_os = "linux")))]
fn init_dpdk(
    _port_id: u16,
    _promiscuous: bool,
) -> Result<Box<dyn DpdkPort>, Box<dyn std::error::Error>> {
    Ok(Box::new(DummyDpdkPort))
}

fn init_pcap(
    source_device: &str,
    #[cfg(target_os = "linux")] _use_wireless: bool,
    #[cfg(not(target_os = "linux"))] use_wireless: bool,
    promiscuous: bool,
    read_time_out: i32,
    read_size: i32,
    immediate_mode: bool,
    buffer_size: i64,
    source_protocol: &str,
    source_port: i32,
    source_ip: &str,
) -> Result<(Capture<Active>, UdpSocket), Box<dyn StdError>> {
    let devices = Device::list().map_err(|e| Box::new(e) as Box<dyn StdError>)?;
    debug!("init_pcap: devices: {:?}", devices);
    info!("init_pcap: specified source_device: {}", source_device);

    // Different handling for Linux and non-Linux systems
    #[cfg(target_os = "linux")]
    let target_device = devices
        .into_iter()
        .find(|d| d.name == source_device || source_device.is_empty())
        .ok_or_else(|| Box::new(DeviceNotFoundError) as Box<dyn StdError>)?;

    #[cfg(not(target_os = "linux"))]
    let target_device = devices
        .into_iter()
        .find(|d| {
            (d.name == source_device || source_device.is_empty())
                && d.flags.is_up()