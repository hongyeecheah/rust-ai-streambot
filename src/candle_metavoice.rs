#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

#[cfg(feature = "metavoice")]
use anyhow::{Error, Result};
#[cfg(feature = "metavoice")]
use bytes::Bytes;
#[cfg(feature = "metavoice")]
use std::io::Cursor;
#[cfg(feature = "metavoice")]
use std::io::Write;

#[cfg(feature = "m