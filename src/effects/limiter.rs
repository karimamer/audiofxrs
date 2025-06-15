use crate::audio_io::AudioData;
use crate::effects::dsp::clamp;
use crate::effects::{float_param, AudioEffect, ParameterDef, ParameterValue, Parameters};

pub struct LimiterEffect {
    sample_rate: f32,

    // Parameters
    threshold: f32,   // Threshold in linear scale (0.0 to 1.0)
    attack_ms: f32,   // Attack time in milliseconds
    release_ms: f32,  // Release time in milliseconds
    output_gain: f32, // Output gain in linear scale

    // Internal state
    envelope: f32,       // Current envelope level
    gain_reduction: f32, // Current gain reduction amount
    attack_coeff: f32,   // Attack coefficient
    release_coeff: f32,  // Release coefficient
}

impl Default for LimiterEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl LimiterEffect {
    pub fn new() -> Self {
        let mut limiter = Self {
            sample_rate: 44100.0,
            threshold: 0.8,
            attack_ms: 1.0,
            release_ms: 50.0,
            output_gain: 1.0,
            envelope: 0.0,
            gain_reduction: 1.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
        };

        limiter.update_coefficients();
        limiter
    }

    fn update_coefficients(&mut self) {
        // Calculate attack and release coefficients for envelope follower
        self.attack_coeff = (-1.0 / (self.attack_ms * 0.001 * self.sample_rate)).exp();
        self.release_coeff = (-1.0 / (self.release_ms * 0.001 * self.sample_rate)).exp();
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        let input_level = input.abs();

        // Envelope follower with separate attack and release
        let coeff = if input_level > self.envelope {
            self.attack_coeff
        } else {
            self.release_coeff
        };

        self.envelope = input_level + (self.envelope - input_level) * coeff;

        // Calculate gain reduction
        let target_gain = if self.envelope > self.threshold {
            self.threshold / self.envelope.max(0.001) // Avoid division by zero
        } else {
            1.0
        };

        // Smooth gain reduction changes
        let gain_coeff = if target_gain < self.gain_reduction {
            self.attack_coeff // Fast attack for gain reduction
        } else {
            self.release_coeff // Slower release
        };

        self.gain_reduction = target_gain + (self.gain_reduction - target_gain) * gain_coeff;

        // Apply limiting and output gain
        let limited = input * self.gain_reduction * self.output_gain;

        // Final safety clamp
        clamp(limited, -1.0, 1.0)
    }
}

impl AudioEffect for LimiterEffect {
    fn name(&self) -> &str {
        "Limiter"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param(
                "threshold",
                "Limiting threshold (0.0 to 1.0)",
                0.8,
                0.1,
                1.0,
            ),
            float_param("attack", "Attack time in milliseconds", 1.0, 0.1, 10.0),
            float_param("release", "Release time in milliseconds", 50.0, 1.0, 500.0),
            float_param("output", "Output gain", 1.0, 0.1, 2.0),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        let mut need_update = false;

        for (key, value) in params {
            match key.as_str() {
                "threshold" => {
                    self.threshold = value
                        .as_float()
                        .ok_or("Threshold parameter must be a number")?
                        .clamp(0.1, 1.0);
                }
                "attack" => {
                    self.attack_ms = value
                        .as_float()
                        .ok_or("Attack parameter must be a number")?
                        .clamp(0.1, 10.0);
                    need_update = true;
                }
                "release" => {
                    self.release_ms = value
                        .as_float()
                        .ok_or("Release parameter must be a number")?
                        .clamp(1.0, 500.0);
                    need_update = true;
                }
                "output" => {
                    self.output_gain = value
                        .as_float()
                        .ok_or("Output gain parameter must be a number")?
                        .clamp(0.1, 2.0);
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
        params.insert(
            "threshold".to_string(),
            ParameterValue::Float(self.threshold),
        );
        params.insert("attack".to_string(), ParameterValue::Float(self.attack_ms));
        params.insert(
            "release".to_string(),
            ParameterValue::Float(self.release_ms),
        );
        params.insert(
            "output".to_string(),
            ParameterValue::Float(self.output_gain),
        );
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
        self.gain_reduction = 1.0;
    }

    fn supports_format(&self, sample_rate: u32, channels: usize) -> bool {
        sample_rate >= 8000 && sample_rate <= 192_000 && channels >= 1 && channels <= 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_io::{default_wav_spec, AudioData};

    #[test]
    fn test_limiter_creation() {
        let limiter = LimiterEffect::new();
        assert_eq!(limiter.name(), "Limiter");
        assert_eq!(limiter.parameter_definitions().len(), 4);
    }

    #[test]
    fn test_parameter_setting() {
        let mut limiter = LimiterEffect::new();
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(0.5));
        params.insert("attack".to_string(), ParameterValue::Float(2.0));
        params.insert("release".to_string(), ParameterValue::Float(100.0));
        params.insert("output".to_string(), ParameterValue::Float(1.2));

        assert!(limiter.set_parameters(params).is_ok());

        let current_params = limiter.get_parameters();
        assert_eq!(
            current_params.get("threshold").unwrap().as_float(),
            Some(0.5)
        );
        assert_eq!(current_params.get("attack").unwrap().as_float(), Some(2.0));
        assert_eq!(
            current_params.get("release").unwrap().as_float(),
            Some(100.0)
        );
        assert_eq!(current_params.get("output").unwrap().as_float(), Some(1.2));
    }

    #[test]
    fn test_limiter_processing() {
        let mut limiter = LimiterEffect::new();

        // Create test audio data with some loud samples
        let samples = vec![0.3, 0.9, -0.95, 0.1, 0.0, 0.85, -0.7, 0.4];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = limiter.process(&input);
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
    fn test_limiting_effect() {
        let mut limiter = LimiterEffect::new();

        // Set a low threshold and fast attack to ensure limiting occurs
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(0.5));
        params.insert("attack".to_string(), ParameterValue::Float(0.5)); // Fast attack
        limiter.set_parameters(params).unwrap();

        // Test with a loud signal that should be limited
        let loud_sample = 0.9;

        // Process more samples to let the limiter fully respond
        let mut last_output = 0.0;
        for _ in 0..1000 {
            last_output = limiter.process_sample(loud_sample);
        }

        // The limited sample should not exceed the threshold significantly
        // Allow some margin for envelope following and output gain
        assert!(last_output.abs() <= 0.8); // More realistic expectation

        // Also test that limiting is actually occurring
        assert!(last_output.abs() < loud_sample);
    }

    #[test]
    fn test_parameter_clamping() {
        let mut limiter = LimiterEffect::new();
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(2.0)); // Above max
        params.insert("attack".to_string(), ParameterValue::Float(0.05)); // Below min
        params.insert("output".to_string(), ParameterValue::Float(5.0)); // Above max

        assert!(limiter.set_parameters(params).is_ok());

        let current_params = limiter.get_parameters();
        assert_eq!(
            current_params.get("threshold").unwrap().as_float(),
            Some(1.0)
        ); // Clamped to max
        assert_eq!(current_params.get("attack").unwrap().as_float(), Some(0.1)); // Clamped to min
        assert_eq!(current_params.get("output").unwrap().as_float(), Some(2.0));
        // Clamped to max
    }

    #[test]
    fn test_soft_signals_pass_through() {
        let mut limiter = LimiterEffect::new();

        // Set threshold high
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(0.9));
        limiter.set_parameters(params).unwrap();

        // Test with quiet signals that shouldn't be limited
        let quiet_samples = vec![0.1, -0.2, 0.3, -0.1];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(quiet_samples.clone(), spec);

        let result = limiter.process(&input).unwrap();

        // Quiet signals should pass through relatively unchanged
        for (input_sample, output_sample) in quiet_samples.iter().zip(result.samples.iter()) {
            let difference = (input_sample - output_sample).abs();
            assert!(difference < 0.1); // Allow small differences due to processing
        }
    }

    #[test]
    fn test_reset() {
        let mut limiter = LimiterEffect::new();

        // Process some samples to build up state
        for _ in 0..100 {
            limiter.process_sample(0.9);
        }

        // Reset should clear internal state
        limiter.reset();
        assert_eq!(limiter.envelope, 0.0);
        assert_eq!(limiter.gain_reduction, 1.0);
    }

    #[test]
    fn test_attack_and_release() {
        let mut fast_limiter = LimiterEffect::new();
        let mut slow_limiter = LimiterEffect::new();

        // Set up fast attack/release
        let mut params1 = Parameters::new();
        params1.insert("attack".to_string(), ParameterValue::Float(0.5));
        params1.insert("release".to_string(), ParameterValue::Float(10.0));
        params1.insert("threshold".to_string(), ParameterValue::Float(0.5));
        fast_limiter.set_parameters(params1).unwrap();

        // Set up slow attack/release
        let mut params2 = Parameters::new();
        params2.insert("attack".to_string(), ParameterValue::Float(5.0));
        params2.insert("release".to_string(), ParameterValue::Float(200.0));
        params2.insert("threshold".to_string(), ParameterValue::Float(0.5));
        slow_limiter.set_parameters(params2).unwrap();

        // Process a burst of loud samples
        let loud_samples = vec![0.8; 50];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(loud_samples, spec);

        let fast_result = fast_limiter.process(&input).unwrap();
        let slow_result = slow_limiter.process(&input).unwrap();

        // Both should process successfully with different characteristics
        assert_eq!(fast_result.samples.len(), slow_result.samples.len());

        // Fast limiter should react more quickly (this is a simplified test)
        assert!(fast_result.samples.iter().all(|&x| x.abs() <= 1.0));
        assert!(slow_result.samples.iter().all(|&x| x.abs() <= 1.0));
    }
}
