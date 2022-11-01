use minimp3::{Decoder, Frame};
use std::io::Cursor;
use std::io::Result;

/// Converts WAV PCM data to f32 samples without explicitly handling error cases in the return type.
///
/// # Arguments
/// * `wav_data` - The bytes of a WAV file.
///
/// # Returns
/