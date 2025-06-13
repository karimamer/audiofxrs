use crate::audio_io::AudioData;
use crate::effects::{AudioEffect, ParameterDef, ParameterValue, Parameters, float_param};
use crate::effects::dsp::{DelayLine, sine_wave};

pub struct VibratoEffect {
    delay_line: DelayLine,
    sample_rate: f32,
    phase: f32,

    // Parameters
    rate_hz: f32,
    depth_ms: f32,
}

impl Default for VibratoEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl VibratoEffect {
    pub fn new() -> Self {
        Self {
            delay_line: DelayLine::new(4410), // 100ms at 44.1kHz
            sample_rate: 44100.0,
            phase: 0.0,
            rate_hz: 5.0,
            depth_ms: 5.0,
        }
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        // Generate LFO
        let lfo = sine_wave(self.phase);
        self.phase += self.rate_hz / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Calculate modulated delay time for pitch modulation
        let base_delay_samples = (self.depth_ms * 0.001 * self.sample_rate) as f32;
        let modulated_delay = base_delay_samples * (1.0 + lfo * 0.5);

        // Write input to delay line
        self.delay_line.write(input);

        // Read modulated delayed sample with interpolation
        self.delay_line.read_interpolated(modulated_delay)
    }
}

impl AudioEffect for VibratoEffect {
    fn name(&self) -> &str {
        "Vibrato"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("rate", "Vibrato rate in Hz", 5.0, 0.1, 20.0),
            float_param("depth", "Modulation depth in milliseconds", 5.0, 0.1, 20.0),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        for (key, value) in params {
            match key.as_str() {
                "rate" => {
                    self.rate_hz = value.as_float()
                        .ok_or("Rate parameter must be a number")?
                        .clamp(0.1, 20.0);
                }
                "depth" => {
                    self.depth_ms = value.as_float()
                        .ok_or("Depth parameter must be a number")?
                        .clamp(0.1, 20.0);
                }
                _ => return Err(format!("Unknown parameter: {}", key)),
            }
        }
        Ok(())
    }

    fn get_parameters(&self) -> Parameters {
        let mut params = Parameters::new();
        params.insert("rate".to_string(), ParameterValue::Float(self.rate_hz));
        params.insert("depth".to_string(), ParameterValue::Float(self.depth_ms));
        params
    }

    fn process(&mut self, input: &AudioData) -> Result<AudioData, String> {
        // Update sample rate if needed
        if self.sample_rate != input.sample_rate as f32 {
            self.sample_rate = input.sample_rate as f32;
            // Recreate delay line with appropriate size for new sample rate
            let max_delay_samples = ((self.depth_ms * 2.0) * 0.001 * self.sample_rate) as usize;
            self.delay_line = DelayLine::new(max_delay_samples.max(1));
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
        self.delay_line.clear();
        self.phase = 0.0;
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
    fn test_vibrato_creation() {
        let vibrato = VibratoEffect::new();
        assert_eq!(vibrato.name(), "Vibrato");
        assert_eq!(vibrato.parameter_definitions().len(), 2);
    }

    #[test]
    fn test_parameter_setting() {
        let mut vibrato = VibratoEffect::new();
        let mut params = Parameters::new();
        params.insert("rate".to_string(), ParameterValue::Float(8.0));
        params.insert("depth".to_string(), ParameterValue::Float(10.0));

        assert!(vibrato.set_parameters(params).is_ok());

        let current_params = vibrato.get_parameters();
        assert_eq!(current_params.get("rate").unwrap().as_float(), Some(8.0));
        assert_eq!(current_params.get("depth").unwrap().as_float(), Some(10.0));
    }

    #[test]
    fn test_vibrato_processing() {
        let mut vibrato = VibratoEffect::new();

        // Create test audio data
        let samples = vec![0.5, -0.3, 0.8, -0.1, 0.0, 0.2];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = vibrato.process(&input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.samples.len(), input.samples.len());
        assert_eq!(output.spec.sample_rate, input.spec.sample_rate);
    }

    #[test]
    fn test_parameter_clamping() {
        let mut vibrato = VibratoEffect::new();
        let mut params = Parameters::new();
        params.insert("rate".to_string(), ParameterValue::Float(100.0)); // Above max
        params.insert("depth".to_string(), ParameterValue::Float(-5.0)); // Below min

        assert!(vibrato.set_parameters(params).is_ok());

        let current_params = vibrato.get_parameters();
        assert_eq!(current_params.get("rate").unwrap().as_float(), Some(20.0)); // Clamped to max
        assert_eq!(current_params.get("depth").unwrap().as_float(), Some(0.1)); // Clamped to min
    }

    #[test]
    fn test_reset() {
        let mut vibrato = VibratoEffect::new();

        // Process some samples to build up state
        let samples = vec![0.5, -0.3, 0.8];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let _result = vibrato.process(&input).unwrap();

        // Reset should clear internal state
        vibrato.reset();
        assert_eq!(vibrato.phase, 0.0);
    }
}
