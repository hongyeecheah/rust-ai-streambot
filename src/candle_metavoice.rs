#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

#[cfg(feature = "metavoice")]
use anyhow::{Error, Result};
#[cfg(feature = "m