use hound::{WavReader, WavWriter, WavSpec, SampleFormat};
use std::path::Path;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AudioError {
    FileNotFound(String),
    InvalidFormat(String),
    IoError(String),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AudioError::FileNotFound(path) => write!(f, "Audio file not found: {}", path),
            AudioError::InvalidFormat(msg) => write!(f, "Invalid audio format: {}", msg),
            AudioError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl Error for AudioError {}

pub struct AudioData {
    pub samples: Vec<f32>,
    pub spec: WavSpec,
    pub num_channels: usize,
    pub sample_rate: u32,
}

impl AudioData {
    pub fn new(samples: Vec<f32>, spec: WavSpec) -> Self {
        Self {
            num_channels: spec.channels as usize,
            sample_rate: spec.sample_rate,
            samples,
            spec,
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn duration_seconds(&self) -> f64 {
        self.len() as f64 / (self.sample_rate as f64 * self.num_channels as f64)
    }
}

/// Read an audio file and return samples as f32 values normalized to [-1.0, 1.0]
pub fn read_audio_file<P: AsRef<Path>>(path: P) -> Result<AudioData, AudioError> {
    let path_str = path.as_ref().to_string_lossy().to_string();

    let mut reader = WavReader::open(&path)
        .map_err(|e| AudioError::FileNotFound(format!("{}: {}", path_str, e)))?;

    let spec = reader.spec();

    // Only support 16-bit PCM for now
    if spec.sample_format != SampleFormat::Int || spec.bits_per_sample != 16 {
        return Err(AudioError::InvalidFormat(
            "Only 16-bit PCM WAV files are supported".to_string()
        ));
    }

    let samples: Result<Vec<f32>, _> = reader
        .samples::<i16>()
        .map(|s| s.map(i16_to_f32))
        .collect();

    let samples = samples
        .map_err(|e| AudioError::IoError(format!("Failed to read samples: {}", e)))?;

    Ok(AudioData::new(samples, spec))
}

/// Write f32 samples to a WAV file
pub fn write_audio_file<P: AsRef<Path>>(
    path: P,
    samples: &[f32],
    spec: WavSpec
) -> Result<(), AudioError> {
    let path_str = path.as_ref().to_string_lossy().to_string();

    let mut writer = WavWriter::create(&path, spec)
        .map_err(|e| AudioError::IoError(format!("Failed to create {}: {}", path_str, e)))?;

    for &sample in samples {
        let sample_i16 = f32_to_i16(sample);
        writer.write_sample(sample_i16)
            .map_err(|e| AudioError::IoError(format!("Failed to write sample: {}", e)))?;
    }

    writer.finalize()
        .map_err(|e| AudioError::IoError(format!("Failed to finalize {}: {}", path_str, e)))?;

    Ok(())
}

/// Convert i16 sample to f32 normalized to [-1.0, 1.0]
pub fn i16_to_f32(sample: i16) -> f32 {
    sample as f32 / 32_768.0
}

/// Convert f32 sample to i16, clamping to valid range
pub fn f32_to_i16(sample: f32) -> i16 {
    (sample * 32_767.0).clamp(-32_768.0, 32_767.0) as i16
}

/// Create a default WAV spec for output files
pub fn default_wav_spec(channels: u16, sample_rate: u32) -> WavSpec {
    WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    }
}

/// Common sample rates
pub mod sample_rates {
    pub const CD_QUALITY: u32 = 44_100;
    pub const HIGH_QUALITY: u32 = 48_000;
    pub const STUDIO_QUALITY: u32 = 96_000;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_conversion() {
        assert_eq!(i16_to_f32(0), 0.0);
        assert_eq!(i16_to_f32(32767), 32767.0 / 32768.0);
        assert_eq!(i16_to_f32(-32768), -1.0);

        assert_eq!(f32_to_i16(0.0), 0);
        assert_eq!(f32_to_i16(1.0), 32767);
        assert_eq!(f32_to_i16(-1.0), -32767);

        // Test clamping
        assert_eq!(f32_to_i16(2.0), 32767);
        assert_eq!(f32_to_i16(-2.0), -32768);
    }

    #[test]
    fn test_default_wav_spec() {
        let spec = default_wav_spec(2, 44100);
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 44100);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(spec.sample_format, SampleFormat::Int);
    }
}
