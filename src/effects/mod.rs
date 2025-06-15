use crate::audio_io::AudioData;
use std::collections::HashMap;

pub mod chorus;
pub mod compression;
pub mod delay;
pub mod distortion;
pub mod eq;
pub mod flanger;
pub mod limiter;
pub mod phaser;
pub mod pitch_shifting;
pub mod reverb;
pub mod time_stretching;
pub mod tremolo;
pub mod vibrato;

/// Common parameter types for audio effects
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    String(String),
}

impl ParameterValue {
    pub fn as_float(&self) -> Option<f32> {
        match self {
            ParameterValue::Float(v) => Some(*v),
            ParameterValue::Int(v) => Some(*v as f32),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i32> {
        match self {
            ParameterValue::Int(v) => Some(*v),
            ParameterValue::Float(v) => Some(*v as i32),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParameterValue::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

/// Parameter definition for effects
#[derive(Debug, Clone)]
pub struct ParameterDef {
    pub name: String,
    pub description: String,
    pub default_value: ParameterValue,
    pub min_value: Option<ParameterValue>,
    pub max_value: Option<ParameterValue>,
}

/// Collection of parameters for an effect
pub type Parameters = HashMap<String, ParameterValue>;

/// Common trait for all audio effects
pub trait AudioEffect {
    /// Get the name of the effect
    fn name(&self) -> &str;

    /// Get parameter definitions for this effect
    fn parameter_definitions(&self) -> Vec<ParameterDef>;

    /// Set effect parameters
    fn set_parameters(&mut self, params: Parameters) -> Result<(), String>;

    /// Get current parameter values
    fn get_parameters(&self) -> Parameters;

    /// Process audio data through the effect
    fn process(&mut self, input: &AudioData) -> Result<AudioData, String>;

    /// Reset the effect's internal state
    fn reset(&mut self);

    /// Check if the effect supports the given sample rate and channel count
    fn supports_format(&self, sample_rate: u32, channels: usize) -> bool {
        // Default implementation supports common formats
        sample_rate >= 8000 && sample_rate <= 192_000 && channels >= 1 && channels <= 8
    }
}

/// Common time-based parameters
pub struct TimeParams {
    pub delay_ms: f32,
    pub feedback: f32,
    pub wet_dry_mix: f32,
}

impl Default for TimeParams {
    fn default() -> Self {
        Self {
            delay_ms: 200.0,
            feedback: 0.5,
            wet_dry_mix: 0.5,
        }
    }
}

/// Common modulation parameters
pub struct ModulationParams {
    pub rate_hz: f32,
    pub depth: f32,
    pub phase: f32,
}

impl Default for ModulationParams {
    fn default() -> Self {
        Self {
            rate_hz: 1.0,
            depth: 0.5,
            phase: 0.0,
        }
    }
}

/// Common filter parameters
pub struct FilterParams {
    pub frequency_hz: f32,
    pub q_factor: f32,
    pub gain_db: f32,
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            frequency_hz: 1000.0,
            q_factor: 1.0,
            gain_db: 0.0,
        }
    }
}

/// Utility functions for parameter creation
pub fn float_param(name: &str, desc: &str, default: f32, min: f32, max: f32) -> ParameterDef {
    ParameterDef {
        name: name.to_string(),
        description: desc.to_string(),
        default_value: ParameterValue::Float(default),
        min_value: Some(ParameterValue::Float(min)),
        max_value: Some(ParameterValue::Float(max)),
    }
}

pub fn int_param(name: &str, desc: &str, default: i32, min: i32, max: i32) -> ParameterDef {
    ParameterDef {
        name: name.to_string(),
        description: desc.to_string(),
        default_value: ParameterValue::Int(default),
        min_value: Some(ParameterValue::Int(min)),
        max_value: Some(ParameterValue::Int(max)),
    }
}

pub fn bool_param(name: &str, desc: &str, default: bool) -> ParameterDef {
    ParameterDef {
        name: name.to_string(),
        description: desc.to_string(),
        default_value: ParameterValue::Bool(default),
        min_value: None,
        max_value: None,
    }
}

/// Common DSP utilities
pub mod dsp {
    use std::f32::consts::PI;

    /// Linear interpolation between two values
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + t * (b - a)
    }

    /// Convert decibels to linear gain
    pub fn db_to_linear(db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }

    /// Convert linear gain to decibels
    pub fn linear_to_db(linear: f32) -> f32 {
        20.0 * linear.log10()
    }

    /// Clamp a value between min and max
    pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
        value.max(min).min(max)
    }

    /// Generate a sine wave sample
    pub fn sine_wave(phase: f32) -> f32 {
        (2.0 * PI * phase).sin()
    }

    /// Soft clipping function using tanh
    pub fn soft_clip(x: f32) -> f32 {
        x.tanh()
    }

    /// Hard clipping function
    pub fn hard_clip(x: f32, threshold: f32) -> f32 {
        clamp(x, -threshold, threshold)
    }

    /// Simple delay line implementation
    pub struct DelayLine {
        buffer: Vec<f32>,
        write_head: usize,
        max_delay_samples: usize,
    }

    impl DelayLine {
        pub fn new(max_delay_samples: usize) -> Self {
            Self {
                buffer: vec![0.0; max_delay_samples],
                write_head: 0,
                max_delay_samples,
            }
        }

        pub fn write(&mut self, sample: f32) {
            self.buffer[self.write_head] = sample;
            self.write_head = (self.write_head + 1) % self.max_delay_samples;
        }

        pub fn read(&self, delay_samples: usize) -> f32 {
            let delay_samples = delay_samples.min(self.max_delay_samples - 1);
            let read_head = (self.write_head + self.max_delay_samples - delay_samples - 1)
                % self.max_delay_samples;
            self.buffer[read_head]
        }

        pub fn read_interpolated(&self, delay_samples: f32) -> f32 {
            let delay_samples = delay_samples.min(self.max_delay_samples as f32 - 1.0);
            let delay_int = delay_samples.floor() as usize;
            let delay_frac = delay_samples.fract();

            let sample1 = self.read(delay_int);
            let sample2 = self.read(delay_int + 1);

            lerp(sample1, sample2, delay_frac)
        }

        pub fn clear(&mut self) {
            self.buffer.fill(0.0);
            self.write_head = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_value_conversions() {
        let float_val = ParameterValue::Float(3.14);
        assert_eq!(float_val.as_float(), Some(3.14));
        assert_eq!(float_val.as_int(), Some(3));

        let int_val = ParameterValue::Int(42);
        assert_eq!(int_val.as_int(), Some(42));
        assert_eq!(int_val.as_float(), Some(42.0));

        let bool_val = ParameterValue::Bool(true);
        assert_eq!(bool_val.as_bool(), Some(true));
        assert_eq!(bool_val.as_float(), None);
    }

    #[test]
    fn test_dsp_utilities() {
        use crate::effects::dsp::*;

        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
        assert_eq!(clamp(-5.0, 0.0, 10.0), 0.0);

        // Test db conversion (approximately)
        let linear = db_to_linear(6.0);
        assert!((linear - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_delay_line() {
        use crate::effects::dsp::DelayLine;

        let mut delay = DelayLine::new(4);

        // Write some samples
        delay.write(1.0);
        delay.write(2.0);
        delay.write(3.0);

        // Read with 2 sample delay
        assert_eq!(delay.read(2), 1.0);
        assert_eq!(delay.read(1), 2.0);
        assert_eq!(delay.read(0), 3.0);
    }
}
