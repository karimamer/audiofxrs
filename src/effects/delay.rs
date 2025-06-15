use crate::audio_io::AudioData;
use crate::effects::dsp::{clamp, DelayLine};
use crate::effects::{float_param, AudioEffect, ParameterDef, ParameterValue, Parameters};

pub struct DelayEffect {
    delay_line: DelayLine,
    sample_rate: f32,

    // Parameters
    delay_time_ms: f32,
    feedback: f32,
    wet_dry_mix: f32,
    damping: f32,

    // Internal state
    low_pass_state: f32,
}

impl Default for DelayEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl DelayEffect {
    pub fn new() -> Self {
        Self {
            delay_line: DelayLine::new(88200), // 2 seconds at 44.1kHz
            sample_rate: 44100.0,
            delay_time_ms: 250.0,
            feedback: 0.3,
            wet_dry_mix: 0.3,
            damping: 0.2,
            low_pass_state: 0.0,
        }
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        // Calculate delay time in samples
        let delay_samples = (self.delay_time_ms * 0.001 * self.sample_rate) as f32;

        // Read delayed sample with interpolation
        let delayed_sample = self.delay_line.read_interpolated(delay_samples);

        // Apply damping (simple low-pass filter) to the feedback signal
        let cutoff = 1.0 - self.damping;
        self.low_pass_state = self.low_pass_state * cutoff + delayed_sample * (1.0 - cutoff);
        let filtered_delayed = self.low_pass_state;

        // Apply feedback
        let feedback_sample = input + filtered_delayed * self.feedback;

        // Write to delay line with clamping to prevent runaway feedback
        let clamped_feedback = clamp(feedback_sample, -1.0, 1.0);
        self.delay_line.write(clamped_feedback);

        // Mix wet and dry signals
        input * (1.0 - self.wet_dry_mix) + delayed_sample * self.wet_dry_mix
    }
}

impl AudioEffect for DelayEffect {
    fn name(&self) -> &str {
        "Delay"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("delay", "Delay time in milliseconds", 250.0, 10.0, 2000.0),
            float_param("feedback", "Feedback amount (0.0 to 0.9)", 0.3, 0.0, 0.9),
            float_param("mix", "Wet/dry mix (0.0 = dry, 1.0 = wet)", 0.3, 0.0, 1.0),
            float_param(
                "damping",
                "High frequency damping of feedback",
                0.2,
                0.0,
                1.0,
            ),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        let mut need_resize = false;

        for (key, value) in params {
            match key.as_str() {
                "delay" => {
                    let new_delay_time = value
                        .as_float()
                        .ok_or("Delay parameter must be a number")?
                        .clamp(10.0, 2000.0);
                    if (new_delay_time - self.delay_time_ms).abs() > 1.0 {
                        self.delay_time_ms = new_delay_time;
                        need_resize = true;
                    }
                }
                "feedback" => {
                    self.feedback = value
                        .as_float()
                        .ok_or("Feedback parameter must be a number")?
                        .clamp(0.0, 0.9);
                }
                "mix" => {
                    self.wet_dry_mix = value
                        .as_float()
                        .ok_or("Mix parameter must be a number")?
                        .clamp(0.0, 1.0);
                }
                "damping" => {
                    self.damping = value
                        .as_float()
                        .ok_or("Damping parameter must be a number")?
                        .clamp(0.0, 1.0);
                }
                _ => return Err(format!("Unknown parameter: {}", key)),
            }
        }

        // Resize delay line if needed
        if need_resize {
            let max_delay_samples =
                ((self.delay_time_ms * 1.2) * 0.001 * self.sample_rate) as usize;
            self.delay_line = DelayLine::new(max_delay_samples.max(1));
        }

        Ok(())
    }

    fn get_parameters(&self) -> Parameters {
        let mut params = Parameters::new();
        params.insert(
            "delay".to_string(),
            ParameterValue::Float(self.delay_time_ms),
        );
        params.insert("feedback".to_string(), ParameterValue::Float(self.feedback));
        params.insert("mix".to_string(), ParameterValue::Float(self.wet_dry_mix));
        params.insert("damping".to_string(), ParameterValue::Float(self.damping));
        params
    }

    fn process(&mut self, input: &AudioData) -> Result<AudioData, String> {
        // Update sample rate if needed
        if self.sample_rate != input.sample_rate as f32 {
            self.sample_rate = input.sample_rate as f32;
            // Recreate delay line with appropriate size for new sample rate
            let max_delay_samples =
                ((self.delay_time_ms * 1.2) * 0.001 * self.sample_rate) as usize;
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
        self.low_pass_state = 0.0;
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
    fn test_delay_creation() {
        let delay = DelayEffect::new();
        assert_eq!(delay.name(), "Delay");
        assert_eq!(delay.parameter_definitions().len(), 4);
    }

    #[test]
    fn test_parameter_setting() {
        let mut delay = DelayEffect::new();
        let mut params = Parameters::new();
        params.insert("delay".to_string(), ParameterValue::Float(500.0));
        params.insert("feedback".to_string(), ParameterValue::Float(0.5));
        params.insert("mix".to_string(), ParameterValue::Float(0.4));
        params.insert("damping".to_string(), ParameterValue::Float(0.3));

        assert!(delay.set_parameters(params).is_ok());

        let current_params = delay.get_parameters();
        assert_eq!(current_params.get("delay").unwrap().as_float(), Some(500.0));
        assert_eq!(
            current_params.get("feedback").unwrap().as_float(),
            Some(0.5)
        );
        assert_eq!(current_params.get("mix").unwrap().as_float(), Some(0.4));
        assert_eq!(current_params.get("damping").unwrap().as_float(), Some(0.3));
    }

    #[test]
    fn test_delay_processing() {
        let mut delay = DelayEffect::new();

        // Set a short delay for testing
        let mut params = Parameters::new();
        params.insert("delay".to_string(), ParameterValue::Float(50.0));
        delay.set_parameters(params).unwrap();

        // Create test audio data
        let samples = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = delay.process(&input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.samples.len(), input.samples.len());
        assert_eq!(output.spec.sample_rate, input.spec.sample_rate);

        // First sample should be mostly the original (with some mix)
        assert!(output.samples[0] > 0.5);

        // Later samples should show the delayed signal
        // (exact timing depends on the delay and sample rate)
    }

    #[test]
    fn test_feedback_stability() {
        let mut delay = DelayEffect::new();

        // Set high feedback but still stable
        let mut params = Parameters::new();
        params.insert("feedback".to_string(), ParameterValue::Float(0.8));
        params.insert("delay".to_string(), ParameterValue::Float(100.0));
        delay.set_parameters(params).unwrap();

        // Process a impulse and many silent samples
        let mut samples = vec![1.0];
        samples.extend(vec![0.0; 1000]);
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = delay.process(&input);
        assert!(result.is_ok());

        let output = result.unwrap();

        // Output should remain bounded despite high feedback
        for &sample in &output.samples {
            assert!(sample >= -2.0 && sample <= 2.0);
        }
    }

    #[test]
    fn test_parameter_clamping() {
        let mut delay = DelayEffect::new();
        let mut params = Parameters::new();
        params.insert("delay".to_string(), ParameterValue::Float(5000.0)); // Above max
        params.insert("feedback".to_string(), ParameterValue::Float(1.5)); // Above max
        params.insert("mix".to_string(), ParameterValue::Float(-0.5)); // Below min

        assert!(delay.set_parameters(params).is_ok());

        let current_params = delay.get_parameters();
        assert_eq!(
            current_params.get("delay").unwrap().as_float(),
            Some(2000.0)
        ); // Clamped to max
        assert_eq!(
            current_params.get("feedback").unwrap().as_float(),
            Some(0.9)
        ); // Clamped to max
        assert_eq!(current_params.get("mix").unwrap().as_float(), Some(0.0)); // Clamped to min
    }

    #[test]
    fn test_wet_dry_mix() {
        let mut delay = DelayEffect::new();

        // Test with 100% dry (no delay effect)
        let mut params = Parameters::new();
        params.insert("mix".to_string(), ParameterValue::Float(0.0));
        delay.set_parameters(params).unwrap();

        let input_samples = vec![0.5, 0.3, 0.8];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(input_samples.clone(), spec);

        let result = delay.process(&input).unwrap();

        // With 100% dry mix, output should be very close to input
        for (i, o) in input_samples.iter().zip(result.samples.iter()) {
            assert!((i - o).abs() < 0.1);
        }
    }

    #[test]
    fn test_reset() {
        let mut delay = DelayEffect::new();

        // Process some samples to build up state
        let samples = vec![1.0, 0.5, -0.3, 0.8];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let _result = delay.process(&input).unwrap();

        // Reset should clear internal state
        delay.reset();
        assert_eq!(delay.low_pass_state, 0.0);
    }

    #[test]
    fn test_damping_effect() {
        let mut delay_no_damping = DelayEffect::new();
        let mut delay_with_damping = DelayEffect::new();

        // Set up one with no damping, one with high damping
        let mut params1 = Parameters::new();
        params1.insert("damping".to_string(), ParameterValue::Float(0.0));
        params1.insert("feedback".to_string(), ParameterValue::Float(0.5));
        delay_no_damping.set_parameters(params1).unwrap();

        let mut params2 = Parameters::new();
        params2.insert("damping".to_string(), ParameterValue::Float(0.8));
        params2.insert("feedback".to_string(), ParameterValue::Float(0.5));
        delay_with_damping.set_parameters(params2).unwrap();

        // Process a bright impulse
        let samples = vec![1.0; 10];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result1 = delay_no_damping.process(&input).unwrap();
        let result2 = delay_with_damping.process(&input).unwrap();

        // Both should process successfully
        assert_eq!(result1.samples.len(), result2.samples.len());
    }
}
