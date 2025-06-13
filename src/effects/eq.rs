use crate::audio_io::AudioData;
use crate::effects::{AudioEffect, ParameterDef, ParameterValue, Parameters, float_param};
use crate::effects::dsp::clamp;

pub struct EqEffect {
    sample_rate: f32,

    // Parameters
    low_gain_db: f32,
    mid_gain_db: f32,
    high_gain_db: f32,
    low_freq: f32,
    high_freq: f32,

    // Filter state variables
    low_filter_state: [f32; 2],
    high_filter_state: [f32; 2],
}

impl Default for EqEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl EqEffect {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            low_gain_db: 0.0,
            mid_gain_db: 0.0,
            high_gain_db: 0.0,
            low_freq: 300.0,
            high_freq: 3000.0,
            low_filter_state: [0.0; 2],
            high_filter_state: [0.0; 2],
        }
    }

    fn db_to_linear(db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        // Simple shelving filters
        let low_cutoff = 2.0 * std::f32::consts::PI * self.low_freq / self.sample_rate;
        let high_cutoff = 2.0 * std::f32::consts::PI * self.high_freq / self.sample_rate;

        // Low shelf filter (simplified)
        let low_coeff = (1.0 - low_cutoff.cos()) / 2.0;
        self.low_filter_state[1] = self.low_filter_state[0];
        self.low_filter_state[0] = input * low_coeff + self.low_filter_state[1] * (1.0 - low_coeff);
        let low_band = self.low_filter_state[0];

        // High shelf filter (simplified)
        let high_coeff = (1.0 - high_cutoff.cos()) / 2.0;
        self.high_filter_state[1] = self.high_filter_state[0];
        self.high_filter_state[0] = input * high_coeff + self.high_filter_state[1] * (1.0 - high_coeff);
        let high_band = input - self.high_filter_state[0];

        // Mid band is what's left
        let mid_band = input - low_band - high_band;

        // Apply gains
        let low_gain = Self::db_to_linear(self.low_gain_db);
        let mid_gain = Self::db_to_linear(self.mid_gain_db);
        let high_gain = Self::db_to_linear(self.high_gain_db);

        let output = low_band * low_gain + mid_band * mid_gain + high_band * high_gain;

        clamp(output, -1.0, 1.0)
    }
}

impl AudioEffect for EqEffect {
    fn name(&self) -> &str {
        "EQ"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("low_gain", "Low frequency gain in dB", 0.0, -12.0, 12.0),
            float_param("mid_gain", "Mid frequency gain in dB", 0.0, -12.0, 12.0),
            float_param("high_gain", "High frequency gain in dB", 0.0, -12.0, 12.0),
            float_param("low_freq", "Low/mid crossover frequency", 300.0, 100.0, 1000.0),
            float_param("high_freq", "Mid/high crossover frequency", 3000.0, 1000.0, 8000.0),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        for (key, value) in params {
            match key.as_str() {
                "low_gain" => {
                    self.low_gain_db = value.as_float()
                        .ok_or("Low gain parameter must be a number")?
                        .clamp(-12.0, 12.0);
                }
                "mid_gain" => {
                    self.mid_gain_db = value.as_float()
                        .ok_or("Mid gain parameter must be a number")?
                        .clamp(-12.0, 12.0);
                }
                "high_gain" => {
                    self.high_gain_db = value.as_float()
                        .ok_or("High gain parameter must be a number")?
                        .clamp(-12.0, 12.0);
                }
                "low_freq" => {
                    self.low_freq = value.as_float()
                        .ok_or("Low frequency parameter must be a number")?
                        .clamp(100.0, 1000.0);
                }
                "high_freq" => {
                    self.high_freq = value.as_float()
                        .ok_or("High frequency parameter must be a number")?
                        .clamp(1000.0, 8000.0);
                }
                _ => return Err(format!("Unknown parameter: {}", key)),
            }
        }
        Ok(())
    }

    fn get_parameters(&self) -> Parameters {
        let mut params = Parameters::new();
        params.insert("low_gain".to_string(), ParameterValue::Float(self.low_gain_db));
        params.insert("mid_gain".to_string(), ParameterValue::Float(self.mid_gain_db));
        params.insert("high_gain".to_string(), ParameterValue::Float(self.high_gain_db));
        params.insert("low_freq".to_string(), ParameterValue::Float(self.low_freq));
        params.insert("high_freq".to_string(), ParameterValue::Float(self.high_freq));
        params
    }

    fn process(&mut self, input: &AudioData) -> Result<AudioData, String> {
        // Update sample rate if needed
        if self.sample_rate != input.sample_rate as f32 {
            self.sample_rate = input.sample_rate as f32;
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
        self.low_filter_state = [0.0; 2];
        self.high_filter_state = [0.0; 2];
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
    fn test_eq_creation() {
        let eq = EqEffect::new();
        assert_eq!(eq.name(), "EQ");
        assert_eq!(eq.parameter_definitions().len(), 5);
    }

    #[test]
    fn test_parameter_setting() {
        let mut eq = EqEffect::new();
        let mut params = Parameters::new();
        params.insert("low_gain".to_string(), ParameterValue::Float(3.0));
        params.insert("mid_gain".to_string(), ParameterValue::Float(-2.0));
        params.insert("high_gain".to_string(), ParameterValue::Float(4.0));

        assert!(eq.set_parameters(params).is_ok());

        let current_params = eq.get_parameters();
        assert_eq!(current_params.get("low_gain").unwrap().as_float(), Some(3.0));
        assert_eq!(current_params.get("mid_gain").unwrap().as_float(), Some(-2.0));
        assert_eq!(current_params.get("high_gain").unwrap().as_float(), Some(4.0));
    }

    #[test]
    fn test_eq_processing() {
        let mut eq = EqEffect::new();

        // Create test audio data
        let samples = vec![0.5, -0.3, 0.8, -0.1, 0.0, 0.2];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = eq.process(&input);
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
    fn test_db_to_linear_conversion() {
        assert!((EqEffect::db_to_linear(0.0) - 1.0).abs() < 0.001);
        assert!((EqEffect::db_to_linear(6.0) - 2.0).abs() < 0.01);
        assert!((EqEffect::db_to_linear(-6.0) - 0.5).abs() < 0.01);
    }
}
