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
        Err(_) => return Ok(Vec::new()), // In case of an error, return an empty vector to match the mp3_to_f32 strategy
    };

    // Depending on the sample format, process the samples differently
    let spec = reader.spec();
    let sample_format = spec.sample_format;
    let bits_per_sample = spec.bits_per_sample;

    let samples = match sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .filter_map(|result_sample| result_sample.ok()) // Convert Result<f32, hound::Error> to Option<f32>, and then filter_map will filter out the None values
            .collect(),

        hound::SampleFormat::Int => match bits_per_sample {
            16