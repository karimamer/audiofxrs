use crate::audio_io::AudioData;
use super::{AudioEffect, ParameterDef, ParameterValue, Parameters, float_param};

pub struct Bitcrusher {
    bit_depth: f32,
    sample_rate_reduction: f32,
    mix: f32,
    sample_accumulator: f32,
    last_crushed_sample: f32,
    samples_since_last_crush: u32,
}

impl Bitcrusher {
    pub fn new() -> Self {
        Self {
            bit_depth: 8.0,
            sample_rate_reduction: 1.0,
            mix: 1.0,
            sample_accumulator: 0.0,
            last_crushed_sample: 0.0,
            samples_since_last_crush: 0,
        }
    }

    fn crush_sample(&mut self, input: f32, original_sample_rate: f32) -> f32 {
        // Calculate how many samples to skip based on sample rate reduction
        let skip_samples = (original_sample_rate / (original_sample_rate / self.sample_rate_reduction)).round() as u32;
        
        self.samples_since_last_crush += 1;
        
        let crushed_sample = if self.samples_since_last_crush >= skip_samples {
            // Time to crush a new sample
            self.samples_since_last_crush = 0;
            
            // Bit depth reduction
            let levels = 2.0_f32.powf(self.bit_depth);
            let quantized = (input * levels * 0.5 + 0.5).floor() / levels * 2.0 - 1.0;
            
            self.last_crushed_sample = quantized;
            quantized
        } else {
            // Use the last crushed sample (sample rate reduction)
            self.last_crushed_sample
        };
        
        // Mix with original signal
        input * (1.0 - self.mix) + crushed_sample * self.mix
    }
}

impl Default for Bitcrusher {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEffect for Bitcrusher {
    fn name(&self) -> &str {
        "Bitcrusher"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("bit_depth", "Bit depth reduction (1-16 bits)", 8.0, 1.0, 16.0),
            float_param("sample_rate_reduction", "Sample rate reduction factor (1-100)", 1.0, 1.0, 100.0),
            float_param("mix", "Dry/Wet mix (0.0 = dry, 1.0 = wet)", 1.0, 0.0, 1.0),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        for (key, value) in params {
            match key.as_str() {
                "bit_depth" => {
                    if let Some(val) = value.as_float() {
                        if val >= 1.0 && val <= 16.0 {
                            self.bit_depth = val;
                        } else {
                            return Err("Bit depth must be between 1.0 and 16.0".to_string());
                        }
                    } else {
                        return Err("Bit depth must be a float".to_string());
                    }
                }
                "sample_rate_reduction" => {
                    if let Some(val) = value.as_float() {
                        if val >= 1.0 && val <= 100.0 {
                            self.sample_rate_reduction = val;
                        } else {
                            return Err("Sample rate reduction must be between 1.0 and 100.0".to_string());
                        }
                    } else {
                        return Err("Sample rate reduction must be a float".to_string());
                    }
                }
                "mix" => {
                    if let Some(val) = value.as_float() {
                        if val >= 0.0 && val <= 1.0 {
                            self.mix = val;
                        } else {
                            return Err("Mix must be between 0.0 and 1.0".to_string());
                        }
                    } else {
                        return Err("Mix must be a float".to_string());
                    }
                }
                _ => return Err(format!("Unknown parameter: {}", key)),
            }
        }
        Ok(())
    }

    fn get_parameters(&self) -> Parameters {
        let mut params = Parameters::new();
        params.insert("bit_depth".to_string(), ParameterValue::Float(self.bit_depth));
        params.insert("sample_rate_reduction".to_string(), ParameterValue::Float(self.sample_rate_reduction));
        params.insert("mix".to_string(), ParameterValue::Float(self.mix));
        params
    }

    fn process(&mut self, input: &AudioData) -> Result<AudioData, String> {
        let mut output = input.clone();
        
        for channel in 0..output.channels {
            for sample_idx in 0..output.samples.len() {
                let input_sample = output.samples[sample_idx][channel];
                let crushed = self.crush_sample(input_sample, output.sample_rate as f32);
                output.samples[sample_idx][channel] = crushed;
            }
        }
        
        Ok(output)
    }

    fn reset(&mut self) {
        self.sample_accumulator = 0.0;
        self.last_crushed_sample = 0.0;
        self.samples_since_last_crush = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_io::AudioData;

    #[test]
    fn test_bitcrusher_creation() {
        let crusher = Bitcrusher::new();
        assert_eq!(crusher.name(), "Bitcrusher");
    }

    #[test]
    fn test_parameter_setting() {
        let mut crusher = Bitcrusher::new();
        let mut params = Parameters::new();
        params.insert("bit_depth".to_string(), ParameterValue::Float(4.0));
        params.insert("sample_rate_reduction".to_string(), ParameterValue::Float(2.0));
        params.insert("mix".to_string(), ParameterValue::Float(0.5));
        
        assert!(crusher.set_parameters(params).is_ok());
        assert_eq!(crusher.bit_depth, 4.0);
        assert_eq!(crusher.sample_rate_reduction, 2.0);
        assert_eq!(crusher.mix, 0.5);
    }

    #[test]
    fn test_invalid_parameters() {
        let mut crusher = Bitcrusher::new();
        let mut params = Parameters::new();
        params.insert("bit_depth".to_string(), ParameterValue::Float(20.0)); // Too high
        
        assert!(crusher.set_parameters(params).is_err());
    }

    #[test]
    fn test_audio_processing() {
        let mut crusher = Bitcrusher::new();
        let input = AudioData {
            samples: vec![vec![0.5, -0.3], vec![0.8, -0.1]],
            sample_rate: 44100,
            channels: 2,
        };
        
        let result = crusher.process(&input);
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert_eq!(output.samples.len(), input.samples.len());
        assert_eq!(output.channels, input.channels);
        assert_eq!(output.sample_rate, input.sample_rate);
    }
}