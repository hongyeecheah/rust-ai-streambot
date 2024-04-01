use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use sysinfo::{NetworkExt, NetworksExt};
use sysinfo::{ProcessorExt, System, SystemExt};

static SYSTEM: Lazy<Mutex<(System, Instant)>> =