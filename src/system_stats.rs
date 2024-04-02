use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use sysinfo::{NetworkExt, NetworksExt};
use sysinfo::{ProcessorExt, System, SystemExt};

static SYSTEM: Lazy<Mutex<(System, Instant)>> = Lazy::new(|| {
    let mut system = System::new_all();
    system.refresh_all(); // Initial refresh
    Mutex::new((system, Instant::now()))
});

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemStats {
    total_memory: u64,
    used_memory: u64,
    total_swap: u64,
    used_swap: u64,
    cpu_usage: f32,
    cpu_count: usize,
    core_count: usize,
    boot_time: u64,
    load_avg: LoadAverage,
    host_name: String,
    kernel_version: String,
    os_version: String,
    networ