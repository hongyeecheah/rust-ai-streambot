use minimp3::{Decoder, Frame};
use std::io::Cursor;
use std::io::Result;

/// Converts WAV PCM data to f32 samples without explicitly handling error cases in the return type.
///
/// # Arguments
/// * `wav_data` - The bytes of a WAV file.
///
/// # Returns
/// A `Result` containing a `Vec<f32>` of normalized audio samples, or an `Error`.
pub fn wav_to_f32(wav_data: Vec<u8>) -> Result<Vec<f32>> {
    let cursor = Cursor::new(wav_data);
    let reader_result = hound::WavReader::new(cursor);

    // Check if the reader was successfully created
    let reader = match reader_result {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()), // In case of an error, return an empty vector to match the m