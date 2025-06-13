use crate::audio_io::AudioData;
use crate::effects::{AudioEffect, ParameterDef, ParameterValue, Parameters, float_param, int_param};
use crate::effects::dsp::{soft_clip, hard_clip, clamp};

#[derive(Debug, Clone, Copy)]
pub enum DistortionType {
    SoftClip,
    HardClip,
    Overdrive,
    Fuzz,
}

impl DistortionType {
    fn from_int(value: i32) -> Self {
        match value {
            0 => DistortionType::SoftClip,
            1 => DistortionType::HardClip,
            2 => DistortionType::Overdrive,
            3 => DistortionType::Fuzz,
            _ => DistortionType::SoftClip,
        }
    }

    fn to_int(self) -> i32 {
        match self {
            DistortionType::SoftClip => 0,
            DistortionType::HardClip => 1,
            DistortionType::Overdrive => 2,
            DistortionType::Fuzz => 3,
        }
    }
}

pub struct DistortionEffect {
    // Parameters
    gain: f32,
    threshold: f32,
    wet_dry_mix: f32,
    output_level: f32,
    distortion_type: DistortionType,
}

impl Default for DistortionEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl DistortionEffect {
    pub fn new() -> Self {
        Self {
            gain: 2.0,
            threshold: 0.7,
            wet_dry_mix: 1.0,
            output_level: 0.8,
            distortion_type: DistortionType::SoftClip,
        }
    }

    fn process_sample(&self, input: f32) -> f32 {
        // Apply input gain
        let gained_sample = input * self.gain;

        // Apply distortion based on type
        let distorted_sample = match self.distortion_type {
            DistortionType::SoftClip => soft_clip(gained_sample),
            DistortionType::HardClip => hard_clip(gained_sample, self.threshold),
            DistortionType::Overdrive => self.overdrive(gained_sample),
            DistortionType::Fuzz => self.fuzz(gained_sample),
        };

        // Mix wet and dry signals
        let mixed_sample = input * (1.0 - self.wet_dry_mix) + distorted_sample * self.wet_dry_mix;

        // Apply output level and clamp
        clamp(mixed_sample * self.output_level, -1.0, 1.0)
    }

    fn overdrive(&self, input: f32) -> f32 {
        // Asymmetric soft clipping for overdrive character
        let abs_input = input.abs();
        if abs_input < self.threshold {
            input
        } else {
            let sign = input.signum();
            let excess = abs_input - self.threshold;
            let compressed = excess / (1.0 + excess * 2.0);
            sign * (self.threshold + compressed)
        }
    }

    fn fuzz(&self, input: f32) -> f32 {
        // Aggressive square-wave-like distortion
        let gained = input * self.gain * 2.0;
        if gained > self.threshold {
            1.0
        } else if gained < -self.threshold {
            -1.0
        } else {
            gained / self.threshold
        }
    }
}

impl AudioEffect for DistortionEffect {
    fn name(&self) -> &str {
        "Distortion"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("gain", "Input gain amount", 2.0, 0.1, 10.0),
            float_param("threshold", "Distortion threshold", 0.7, 0.1, 1.0),
            float_param("mix", "Wet/dry mix (0.0 = dry, 1.0 = wet)", 1.0, 0.0, 1.0),
            float_param("output", "Output level", 0.8, 0.1, 1.0),
            int_param("type", "Distortion type (0=Soft, 1=Hard, 2=Overdrive, 3=Fuzz)", 0, 0, 3),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        for (key, value) in params {
            match key.as_str() {
                "gain" => {
                    self.gain = value.as_float()
                        .ok_or("Gain parameter must be a number")?
                        .clamp(0.1, 10.0);
                }
                "threshold" => {
                    self.threshold = value.as_float()
                        .ok_or("Threshold parameter must be a number")?
                        .clamp(0.1, 1.0);
                }
                "mix" => {
                    self.wet_dry_mix = value.as_float()
                        .ok_or("Mix parameter must be a number")?
                        .clamp(0.0, 1.0);
                }
                "output" => {
                    self.output_level = value.as_float()
                        .ok_or("Output parameter must be a number")?
                        .clamp(0.1, 1.0);
                }
                "type" => {
                    let type_int = value.as_int()
                        .ok_or("Type parameter must be an integer")?
                        .clamp(0, 3);
                    self.distortion_type = DistortionType::from_int(type_int);
                }
                _ => return Err(format!("Unknown parameter: {}", key)),
            }
        }
        Ok(())
    }

    fn get_parameters(&self) -> Parameters {
        let mut params = Parameters::new();
        params.insert("gain".to_string(), ParameterValue::Float(self.gain));
        params.insert("threshold".to_string(), ParameterValue::Float(self.threshold));
        params.insert("mix".to_string(), ParameterValue::Float(self.wet_dry_mix));
        params.insert("output".to_string(), ParameterValue::Float(self.output_level));
        params.insert("type".to_string(), ParameterValue::Int(self.distortion_type.to_int()));
        params
    }

    fn process(&mut self, input: &AudioData) -> Result<AudioData, String> {
        let mut output_samples = Vec::with_capacity(input.samples.len());

        // Process each sample
        for &sample in &input.samples {
            let processed = self.process_sample(sample);
            output_samples.push(processed);
        }

        Ok(AudioData::new(output_samples, input.spec))
    }

    fn reset(&mut self) {
        // Distortion is stateless, so nothing to reset
    }

    fn supports_format(&self, sample_rate: u32, channels: usize) -> bool {
        sample_rate >= 8000 && sample_rate <= 192_000 && channels >= 1 && channels <= 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_io::{AudioData, default_wav_spec};

    #[test]
    fn test_distortion_creation() {
        let distortion = DistortionEffect::new();
        assert_eq!(distortion.name(), "Distortion");
        assert_eq!(distortion.parameter_definitions().len(), 5);
    }

    #[test]
    fn test_parameter_setting() {
        let mut distortion = DistortionEffect::new();
        let mut params = Parameters::new();
        params.insert("gain".to_string(), ParameterValue::Float(3.0));
        params.insert("threshold".to_string(), ParameterValue::Float(0.5));
        params.insert("type".to_string(), ParameterValue::Int(1));

        assert!(distortion.set_parameters(params).is_ok());

        let current_params = distortion.get_parameters();
        assert_eq!(current_params.get("gain").unwrap().as_float(), Some(3.0));
        assert_eq!(current_params.get("threshold").unwrap().as_float(), Some(0.5));
        assert_eq!(current_params.get("type").unwrap().as_int(), Some(1));
    }

    #[test]
    fn test_distortion_types() {
        assert_eq!(DistortionType::from_int(0).to_int(), 0);
        assert_eq!(DistortionType::from_int(3).to_int(), 3);
        assert_eq!(DistortionType::from_int(99).to_int(), 0); // Should default to SoftClip
    }

    #[test]
    fn test_distortion_processing() {
        let mut distortion = DistortionEffect::new();

        // Create test audio data with some loud samples
        let samples = vec![0.5, -0.8, 1.2, -1.5, 0.0, 0.9];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = distortion.process(&input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.samples.len(), input.samples.len());

        // Output should be clamped to [-1.0, 1.0]
        for &sample in &output.samples {
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_soft_vs_hard_clipping() {
        let mut soft_distortion = DistortionEffect::new();
        let mut hard_distortion = DistortionEffect::new();

        // Set up hard clipping
        let mut params = Parameters::new();
        params.insert("type".to_string(), ParameterValue::Int(1)); // Hard clip
        hard_distortion.set_parameters(params).unwrap();

        // Test with a sample that should be clipped differently
        let test_sample = 0.9;
        let soft_result = soft_distortion.process_sample(test_sample);
        let hard_result = hard_distortion.process_sample(test_sample);

        // Soft clipping should be more gradual than hard clipping
        assert_ne!(soft_result, hard_result);
    }

    #[test]
    fn test_wet_dry_mix() {
        let mut distortion = DistortionEffect::new();

        // Set to 100% dry (no distortion)
        let mut params = Parameters::new();
        params.insert("mix".to_string(), ParameterValue::Float(0.0));
        distortion.set_parameters(params).unwrap();

        let input_sample = 0.5;
        let dry_result = distortion.process_sample(input_sample);

        // Should be close to original (accounting for output level)
        assert!((dry_result - input_sample * distortion.output_level).abs() < 0.01);

        // Set to 100% wet (full distortion)
        let mut params = Parameters::new();
        params.insert("mix".to_string(), ParameterValue::Float(1.0));
        distortion.set_parameters(params).unwrap();

        let wet_result = distortion.process_sample(input_sample);

        // Should be different from dry result
        assert_ne!(dry_result, wet_result);
    }
}
