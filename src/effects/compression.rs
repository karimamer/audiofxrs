use crate::audio_io::AudioData;
use crate::effects::{AudioEffect, ParameterDef, ParameterValue, Parameters, float_param};
use crate::effects::dsp::clamp;

pub struct CompressionEffect {
    sample_rate: f32,

    // Parameters
    threshold: f32,      // Threshold in linear scale (0.0 to 1.0)
    ratio: f32,          // Compression ratio (1.0 to 20.0)
    attack_ms: f32,      // Attack time in milliseconds
    release_ms: f32,     // Release time in milliseconds
    makeup_gain: f32,    // Makeup gain in linear scale

    // Internal state
    envelope: f32,       // Current envelope level
    attack_coeff: f32,   // Attack coefficient
    release_coeff: f32,  // Release coefficient
}

impl Default for CompressionEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressionEffect {
    pub fn new() -> Self {
        let mut compressor = Self {
            sample_rate: 44100.0,
            threshold: 0.5,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            makeup_gain: 1.0,
            envelope: 0.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
        };

        compressor.update_coefficients();
        compressor
    }

    fn update_coefficients(&mut self) {
        // Calculate attack and release coefficients
        self.attack_coeff = (-1.0 / (self.attack_ms * 0.001 * self.sample_rate)).exp();
        self.release_coeff = (-1.0 / (self.release_ms * 0.001 * self.sample_rate)).exp();
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        let input_level = input.abs();

        // Envelope follower
        let coeff = if input_level > self.envelope {
            self.attack_coeff
        } else {
            self.release_coeff
        };

        self.envelope = input_level + (self.envelope - input_level) * coeff;

        // Calculate gain reduction
        let gain_reduction = if self.envelope > self.threshold {
            let over_threshold = self.envelope - self.threshold;
            let compressed_over = over_threshold / self.ratio;
            let target_level = self.threshold + compressed_over;
            if self.envelope > 0.0 {
                target_level / self.envelope
            } else {
                1.0
            }
        } else {
            1.0
        };

        // Apply compression and makeup gain
        let compressed = input * gain_reduction * self.makeup_gain;

        // Clamp to prevent clipping
        clamp(compressed, -1.0, 1.0)
    }
}

impl AudioEffect for CompressionEffect {
    fn name(&self) -> &str {
        "Compression"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("threshold", "Compression threshold (0.0 to 1.0)", 0.5, 0.0, 1.0),
            float_param("ratio", "Compression ratio", 4.0, 1.0, 20.0),
            float_param("attack", "Attack time in milliseconds", 10.0, 0.1, 100.0),
            float_param("release", "Release time in milliseconds", 100.0, 10.0, 1000.0),
            float_param("makeup", "Makeup gain", 1.0, 0.1, 4.0),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        let mut need_update = false;

        for (key, value) in params {
            match key.as_str() {
                "threshold" => {
                    self.threshold = value.as_float()
                        .ok_or("Threshold parameter must be a number")?
                        .clamp(0.0, 1.0);
                }
                "ratio" => {
                    self.ratio = value.as_float()
                        .ok_or("Ratio parameter must be a number")?
                        .clamp(1.0, 20.0);
                }
                "attack" => {
                    self.attack_ms = value.as_float()
                        .ok_or("Attack parameter must be a number")?
                        .clamp(0.1, 100.0);
                    need_update = true;
                }
                "release" => {
                    self.release_ms = value.as_float()
                        .ok_or("Release parameter must be a number")?
                        .clamp(10.0, 1000.0);
                    need_update = true;
                }
                "makeup" => {
                    self.makeup_gain = value.as_float()
                        .ok_or("Makeup gain parameter must be a number")?
                        .clamp(0.1, 4.0);
                }
                _ => return Err(format!("Unknown parameter: {}", key)),
            }
        }

        if need_update {
            self.update_coefficients();
        }

        Ok(())
    }

    fn get_parameters(&self) -> Parameters {
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(self.threshold));
        params.insert("ratio".to_string(), ParameterValue::Float(self.ratio));
        params.insert("attack".to_string(), ParameterValue::Float(self.attack_ms));
        params.insert("release".to_string(), ParameterValue::Float(self.release_ms));
        params.insert("makeup".to_string(), ParameterValue::Float(self.makeup_gain));
        params
    }

    fn process(&mut self, input: &AudioData) -> Result<AudioData, String> {
        // Update sample rate if needed
        if self.sample_rate != input.sample_rate as f32 {
            self.sample_rate = input.sample_rate as f32;
            self.update_coefficients();
        }

        let mut output_samples = Vec::with_capacity(input.samples.len());

        // Process each sample
        for &sample in &input.samples {
            let processed = self.process_sample(sample);
            output_samples.push(processed);
        }

        Ok(AudioData::new(output_samples, input.spec))
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
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
    fn test_compression_creation() {
        let compressor = CompressionEffect::new();
        assert_eq!(compressor.name(), "Compression");
        assert_eq!(compressor.parameter_definitions().len(), 5);
    }

    #[test]
    fn test_parameter_setting() {
        let mut compressor = CompressionEffect::new();
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(0.3));
        params.insert("ratio".to_string(), ParameterValue::Float(8.0));
        params.insert("attack".to_string(), ParameterValue::Float(5.0));

        assert!(compressor.set_parameters(params).is_ok());

        let current_params = compressor.get_parameters();
        assert_eq!(current_params.get("threshold").unwrap().as_float(), Some(0.3));
        assert_eq!(current_params.get("ratio").unwrap().as_float(), Some(8.0));
        assert_eq!(current_params.get("attack").unwrap().as_float(), Some(5.0));
    }

    #[test]
    fn test_compression_processing() {
        let mut compressor = CompressionEffect::new();

        // Create test audio data with some loud samples
        let samples = vec![0.2, 0.8, -0.9, 0.1, 0.0, 0.7, -0.6, 0.3];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = compressor.process(&input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.samples.len(), input.samples.len());
        assert_eq!(output.spec.sample_rate, input.spec.sample_rate);

        // Output should be clamped to [-1.0, 1.0]
        for &sample in &output.samples {
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_compression_effect() {
        let mut compressor = CompressionEffect::new();

        // Set a low threshold to ensure compression occurs
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(0.3));
        params.insert("ratio".to_string(), ParameterValue::Float(4.0));
        compressor.set_parameters(params).unwrap();

        // Test with a loud signal that should be compressed
        let loud_sample = 0.8;
        let compressed = compressor.process_sample(loud_sample);

        // The compressed sample should be quieter than the original
        assert!(compressed.abs() < loud_sample);
    }

    #[test]
    fn test_parameter_clamping() {
        let mut compressor = CompressionEffect::new();
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(2.0)); // Above max
        params.insert("ratio".to_string(), ParameterValue::Float(0.5)); // Below min

        assert!(compressor.set_parameters(params).is_ok());

        let current_params = compressor.get_parameters();
        assert_eq!(current_params.get("threshold").unwrap().as_float(), Some(1.0)); // Clamped to max
        assert_eq!(current_params.get("ratio").unwrap().as_float(), Some(1.0)); // Clamped to min
    }
}
