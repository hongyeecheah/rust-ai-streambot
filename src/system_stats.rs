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

#[derive(Serialize, 