use crate::audio_io::AudioData;
use crate::effects::{AudioEffect, ParameterDef, ParameterValue, Parameters, float_param};

pub struct TimeStretchingEffect {
    // Parameters
    time_stretch_factor: f32, // 1.0 = no change, 2.0 = twice as long, 0.5 = half as long
    wet_dry_mix: f32,
}

impl Default for TimeStretchingEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeStretchingEffect {
    pub fn new() -> Self {
        Self {
            time_stretch_factor: 1.0,
            wet_dry_mix: 1.0,
        }
    }

    fn process_sample(&self, input: f32) -> f32 {
        // TODO: Implement actual time stretching algorithm
        // For now, just pass through the input
        // Real implementation would use techniques like:
        // - PSOLA (Pitch Synchronous Overlap and Add)
        // - Phase vocoder
        // - Granular synthesis
        // - WSOLA (Waveform Similarity Overlap-Add)
        input * self.wet_dry_mix + input * (1.0 - self.wet_dry_mix)
    }
}

impl AudioEffect for TimeStretchingEffect {
    fn name(&self) -> &str {
        "Time Stretching"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("stretch", "Time stretch factor (1.0 = no change, 2.0 = twice as long)", 1.0, 0.25, 4.0),
            float_param("mix", "Wet/dry mix (0.0 = dry, 1.0 = wet)", 1.0, 0.0, 1.0),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        for (key, value) in params {
            match key.as_str() {
                "stretch" => {
                    self.time_stretch_factor = value.as_float()
                        .ok_or("Stretch parameter must be a number")?
                        .clamp(0.25, 4.0);
                }
                "mix" => {
                    self.wet_dry_mix = value.as_float()
                        .ok_or("Mix parameter must be a number")?
                        .clamp(0.0, 1.0);
                }
                _ => return Err(format!("Unknown parameter: {}", key)),
            }
        }
        Ok(())
    }

    fn get_parameters(&self) -> Parameters {
        let mut params = Parameters::new();
        params.insert("stretch".to_string(), ParameterValue::Float(self.time_stretch_factor));
        params.insert("mix".to_string(), ParameterValue::Float(self.wet_dry_mix));
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
        // No internal state to reset in this basic implementation
    }

    fn supports_format(&self, sample_rate: u32, channels: usize) -> bool {
        sample_rate >= 8000 && sample_rate <= 192_000 && channels >= 1 && channels <= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_io::{AudioData, default_wav_spec};

    #[test]
    fn test_time_stretching_creation() {
        let time_stretch = TimeStretchingEffect::new();
        assert_eq!(time_stretch.name(), "Time Stretching");
        assert_eq!(time_stretch.parameter_definitions().len(), 2);
    }

    #[test]
    fn test_parameter_setting() {
        let mut time_stretch = TimeStretchingEffect::new();
        let mut params = Parameters::new();
        params.insert("stretch".to_string(), ParameterValue::Float(2.0));
        params.insert("mix".to_string(), ParameterValue::Float(0.8));

        assert!(time_stretch.set_parameters(params).is_ok());

        let current_params = time_stretch.get_parameters();
        assert_eq!(current_params.get("stretch").unwrap().as_float(), Some(2.0));
        assert_eq!(current_params.get("mix").unwrap().as_float(), Some(0.8));
    }

    #[test]
    fn test_time_stretching_processing() {
        let mut time_stretch = TimeStretchingEffect::new();

        // Create test audio data
        let samples = vec![0.5, -0.3, 0.8, -0.1, 0.0, 0.2];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = time_stretch.process(&input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.samples.len(), input.samples.len());
        assert_eq!(output.spec.sample_rate, input.spec.sample_rate);
    }

    #[test]
    fn test_parameter_clamping() {
        let mut time_stretch = TimeStretchingEffect::new();
        let mut params = Parameters::new();
        params.insert("stretch".to_string(), ParameterValue::Float(10.0)); // Above max
        params.insert("mix".to_string(), ParameterValue::Float(-0.5)); // Below min

        assert!(time_stretch.set_parameters(params).is_ok());

        let current_params = time_stretch.get_parameters();
        assert_eq!(current_params.get("stretch").unwrap().as_float(), Some(4.0)); // Clamped to max
        assert_eq!(current_params.get("mix").unwrap().as_float(), Some(0.0)); // Clamped to min
    }

    #[test]
    fn test_passthrough_behavior() {
        let mut time_stretch = TimeStretchingEffect::new();

        // With default parameters (stretch = 1.0, mix = 1.0), should pass through
        let input_sample = 0.5;
        let output_sample = time_stretch.process_sample(input_sample);

        // Should be the same (or very close) for pass-through
        assert!((output_sample - input_sample).abs() < 0.001);
    }

    #[test]
    fn test_stretch_factor_validation() {
        let mut time_stretch = TimeStretchingEffect::new();

        // Test various stretch factors
        let test_factors = vec![0.5, 1.0, 1.5, 2.0];

        for factor in test_factors {
            let mut params = Parameters::new();
            params.insert("stretch".to_string(), ParameterValue::Float(factor));

            assert!(time_stretch.set_parameters(params).is_ok());

            let current_params = time_stretch.get_parameters();
            assert_eq!(current_params.get("stretch").unwrap().as_float(), Some(factor));
        }
    }
}
