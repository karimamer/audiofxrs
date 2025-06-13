use crate::audio_io::AudioData;
use crate::effects::{AudioEffect, ParameterDef, ParameterValue, Parameters, float_param};
use crate::effects::dsp::{DelayLine, sine_wave};

pub struct FlangerEffect {
    delay_line: DelayLine,
    sample_rate: f32,
    phase: f32,

    // Parameters
    rate_hz: f32,
    depth_ms: f32,
    feedback: f32,
    wet_dry_mix: f32,
}

impl Default for FlangerEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl FlangerEffect {
    pub fn new() -> Self {
        Self {
            delay_line: DelayLine::new(4410), // 100ms at 44.1kHz
            sample_rate: 44100.0,
            phase: 0.0,
            rate_hz: 0.5,
            depth_ms: 2.0,
            feedback: 0.5,
            wet_dry_mix: 0.5,
        }
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        // Generate LFO
        let lfo = sine_wave(self.phase);
        self.phase += self.rate_hz / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Calculate modulated delay time (shorter than chorus)
        let base_delay_samples = (self.depth_ms * 0.001 * self.sample_rate) as f32;
        let modulated_delay = base_delay_samples * (0.5 + lfo * 0.5);

        // Read delayed sample with interpolation
        let delayed_sample = self.delay_line.read_interpolated(modulated_delay);

        // Apply feedback
        let feedback_sample = input + delayed_sample * self.feedback;
        self.delay_line.write(feedback_sample);

        // Mix wet and dry signals (flanger typically adds the delayed signal)
        input + delayed_sample * self.wet_dry_mix
    }
}

impl AudioEffect for FlangerEffect {
    fn name(&self) -> &str {
        "Flanger"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("rate", "LFO rate in Hz", 0.5, 0.1, 10.0),
            float_param("depth", "Modulation depth in milliseconds", 2.0, 0.1, 10.0),
            float_param("feedback", "Feedback amount", 0.5, 0.0, 0.9),
            float_param("mix", "Wet/dry mix", 0.5, 0.0, 1.0),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        for (key, value) in params {
            match key.as_str() {
                "rate" => {
                    self.rate_hz = value.as_float()
                        .ok_or("Rate parameter must be a number")?
                        .clamp(0.1, 10.0);
                }
                "depth" => {
                    self.depth_ms = value.as_float()
                        .ok_or("Depth parameter must be a number")?
                        .clamp(0.1, 10.0);
                }
                "feedback" => {
                    self.feedback = value.as_float()
                        .ok_or("Feedback parameter must be a number")?
                        .clamp(0.0, 0.9);
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
        params.insert("rate".to_string(), ParameterValue::Float(self.rate_hz));
        params.insert("depth".to_string(), ParameterValue::Float(self.depth_ms));
        params.insert("feedback".to_string(), ParameterValue::Float(self.feedback));
        params.insert("mix".to_string(), ParameterValue::Float(self.wet_dry_mix));
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
    fn test_flanger_creation() {
        let flanger = FlangerEffect::new();
        assert_eq!(flanger.name(), "Flanger");
        assert_eq!(flanger.parameter_definitions().len(), 4);
    }

    #[test]
    fn test_parameter_setting() {
        let mut flanger = FlangerEffect::new();
        let mut params = Parameters::new();
        params.insert("rate".to_string(), ParameterValue::Float(1.0));
        params.insert("depth".to_string(), ParameterValue::Float(3.0));
        params.insert("feedback".to_string(), ParameterValue::Float(0.7));

        assert!(flanger.set_parameters(params).is_ok());

        let current_params = flanger.get_parameters();
        assert_eq!(current_params.get("rate").unwrap().as_float(), Some(1.0));
        assert_eq!(current_params.get("depth").unwrap().as_float(), Some(3.0));
        assert_eq!(current_params.get("feedback").unwrap().as_float(), Some(0.7));
    }

    #[test]
    fn test_flanger_processing() {
        let mut flanger = FlangerEffect::new();

        // Create test audio data
        let samples = vec![0.5, -0.3, 0.8, -0.1, 0.0, 0.2];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = flanger.process(&input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.samples.len(), input.samples.len());
        assert_eq!(output.spec.sample_rate, input.spec.sample_rate);
    }

    #[test]
    fn test_parameter_clamping() {
        let mut flanger = FlangerEffect::new();
        let mut params = Parameters::new();
        params.insert("rate".to_string(), ParameterValue::Float(100.0)); // Above max
        params.insert("feedback".to_string(), ParameterValue::Float(-0.5)); // Below min

        assert!(flanger.set_parameters(params).is_ok());

        let current_params = flanger.get_parameters();
        assert_eq!(current_params.get("rate").unwrap().as_float(), Some(10.0)); // Clamped to max
        assert_eq!(current_params.get("feedback").unwrap().as_float(), Some(0.0)); // Clamped to min
    }

    #[test]
    fn test_reset() {
        let mut flanger = FlangerEffect::new();

        // Process some samples to build up state
        let samples = vec![0.5, -0.3, 0.8];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let _result = flanger.process(&input).unwrap();

        // Reset should clear internal state
        flanger.reset();
        assert_eq!(flanger.phase, 0.0);
    }
}
