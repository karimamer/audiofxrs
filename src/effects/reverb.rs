use crate::audio_io::AudioData;
use crate::effects::dsp::DelayLine;
use crate::effects::{float_param, AudioEffect, ParameterDef, ParameterValue, Parameters};

pub struct ReverbEffect {
    delay_lines: Vec<DelayLine>,
    sample_rate: f32,

    // Parameters
    room_size: f32,
    damping: f32,
    wet_dry_mix: f32,
    feedback: f32,
    pre_delay_ms: f32,

    // Internal state
    pre_delay_line: DelayLine,
    low_pass_state: f32,
}

impl Default for ReverbEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl ReverbEffect {
    pub fn new() -> Self {
        Self {
            delay_lines: Vec::new(),
            sample_rate: 44100.0,
            room_size: 0.5,
            damping: 0.5,
            wet_dry_mix: 0.3,
            feedback: 0.5,
            pre_delay_ms: 20.0,
            pre_delay_line: DelayLine::new(4410), // 100ms at 44.1kHz
            low_pass_state: 0.0,
        }
    }

    fn initialize_delay_lines(&mut self) {
        // Create delay lines with prime number lengths for natural diffusion
        let base_delay_ms = 30.0 * self.room_size;
        let delay_times_ms = vec![
            base_delay_ms * 1.0,
            base_delay_ms * 1.3,
            base_delay_ms * 1.7,
            base_delay_ms * 2.1,
            base_delay_ms * 2.7,
            base_delay_ms * 3.1,
        ];

        self.delay_lines.clear();
        for &delay_ms in &delay_times_ms {
            let delay_samples = ((delay_ms * 0.001 * self.sample_rate) as usize).max(1);
            self.delay_lines.push(DelayLine::new(delay_samples));
        }

        // Update pre-delay line
        let pre_delay_samples = ((self.pre_delay_ms * 0.001 * self.sample_rate) as usize).max(1);
        self.pre_delay_line = DelayLine::new(pre_delay_samples);
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        // Apply pre-delay
        self.pre_delay_line.write(input);
        let pre_delayed = self
            .pre_delay_line
            .read((self.pre_delay_ms * 0.001 * self.sample_rate) as usize);

        // Process through delay lines
        let mut reverb_sum = 0.0;
        let num_lines = self.delay_lines.len() as f32;

        for (i, delay_line) in self.delay_lines.iter_mut().enumerate() {
            // Read from delay line
            let delayed = delay_line.read(0);

            // Apply damping inline (simple low-pass filter)
            let cutoff = 1.0 - self.damping;
            self.low_pass_state = self.low_pass_state * cutoff + delayed * (1.0 - cutoff);
            let damped = self.low_pass_state;

            // Mix input with feedback
            let feedback_amount = self.feedback * (1.0 - i as f32 * 0.1 / num_lines);
            let delay_input = pre_delayed + damped * feedback_amount;

            // Write to delay line
            delay_line.write(delay_input);

            // Add to reverb sum with slight phase inversion for some channels
            let phase_mult = if i % 2 == 0 { 1.0 } else { -0.8 };
            reverb_sum += damped * phase_mult;
        }

        // Normalize reverb sum
        reverb_sum /= num_lines;

        // Mix wet and dry signals
        input * (1.0 - self.wet_dry_mix) + reverb_sum * self.wet_dry_mix
    }

    fn apply_damping(&mut self, input: f32) -> f32 {
        // Simple one-pole low-pass filter for damping
        let cutoff = 1.0 - self.damping;
        self.low_pass_state = self.low_pass_state * cutoff + input * (1.0 - cutoff);
        self.low_pass_state
    }
}

impl AudioEffect for ReverbEffect {
    fn name(&self) -> &str {
        "Reverb"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param(
                "room_size",
                "Room size (0.0 = small, 1.0 = large)",
                0.5,
                0.1,
                1.0,
            ),
            float_param("damping", "High frequency damping", 0.5, 0.0, 1.0),
            float_param("mix", "Wet/dry mix (0.0 = dry, 1.0 = wet)", 0.3, 0.0, 1.0),
            float_param("feedback", "Feedback amount", 0.5, 0.0, 0.9),
            float_param(
                "pre_delay",
                "Pre-delay time in milliseconds",
                20.0,
                0.0,
                100.0,
            ),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        let mut need_reinit = false;

        for (key, value) in params {
            match key.as_str() {
                "room_size" => {
                    let new_size = value
                        .as_float()
                        .ok_or("Room size parameter must be a number")?
                        .clamp(0.1, 1.0);
                    if (new_size - self.room_size).abs() > 0.01 {
                        self.room_size = new_size;
                        need_reinit = true;
                    }
                }
                "damping" => {
                    self.damping = value
                        .as_float()
                        .ok_or("Damping parameter must be a number")?
                        .clamp(0.0, 1.0);
                }
                "mix" => {
                    self.wet_dry_mix = value
                        .as_float()
                        .ok_or("Mix parameter must be a number")?
                        .clamp(0.0, 1.0);
                }
                "feedback" => {
                    self.feedback = value
                        .as_float()
                        .ok_or("Feedback parameter must be a number")?
                        .clamp(0.0, 0.9);
                }
                "pre_delay" => {
                    let new_pre_delay = value
                        .as_float()
                        .ok_or("Pre-delay parameter must be a number")?
                        .clamp(0.0, 100.0);
                    if (new_pre_delay - self.pre_delay_ms).abs() > 0.1 {
                        self.pre_delay_ms = new_pre_delay;
                        need_reinit = true;
                    }
                }
                _ => return Err(format!("Unknown parameter: {}", key)),
            }
        }

        if need_reinit {
            self.initialize_delay_lines();
        }

        Ok(())
    }

    fn get_parameters(&self) -> Parameters {
        let mut params = Parameters::new();
        params.insert(
            "room_size".to_string(),
            ParameterValue::Float(self.room_size),
        );
        params.insert("damping".to_string(), ParameterValue::Float(self.damping));
        params.insert("mix".to_string(), ParameterValue::Float(self.wet_dry_mix));
        params.insert("feedback".to_string(), ParameterValue::Float(self.feedback));
        params.insert(
            "pre_delay".to_string(),
            ParameterValue::Float(self.pre_delay_ms),
        );
        params
    }

    fn process(&mut self, input: &AudioData) -> Result<AudioData, String> {
        // Update sample rate if needed
        if self.sample_rate != input.sample_rate as f32 {
            self.sample_rate = input.sample_rate as f32;
            self.initialize_delay_lines();
        }

        // Initialize delay lines if not done yet
        if self.delay_lines.is_empty() {
            self.initialize_delay_lines();
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
        for delay_line in &mut self.delay_lines {
            delay_line.clear();
        }
        self.pre_delay_line.clear();
        self.low_pass_state = 0.0;
    }

    fn supports_format(&self, sample_rate: u32, channels: usize) -> bool {
        sample_rate >= 8000 && sample_rate <= 192_000 && channels >= 1 && channels <= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_io::{default_wav_spec, AudioData};

    #[test]
    fn test_reverb_creation() {
        let reverb = ReverbEffect::new();
        assert_eq!(reverb.name(), "Reverb");
        assert_eq!(reverb.parameter_definitions().len(), 5);
    }

    #[test]
    fn test_parameter_setting() {
        let mut reverb = ReverbEffect::new();
        let mut params = Parameters::new();
        params.insert("room_size".to_string(), ParameterValue::Float(0.8));
        params.insert("damping".to_string(), ParameterValue::Float(0.3));
        params.insert("mix".to_string(), ParameterValue::Float(0.6));

        assert!(reverb.set_parameters(params).is_ok());

        let current_params = reverb.get_parameters();
        assert_eq!(
            current_params.get("room_size").unwrap().as_float(),
            Some(0.8)
        );
        assert_eq!(current_params.get("damping").unwrap().as_float(), Some(0.3));
        assert_eq!(current_params.get("mix").unwrap().as_float(), Some(0.6));
    }

    #[test]
    fn test_reverb_processing() {
        let mut reverb = ReverbEffect::new();

        // Create test audio data
        let samples = vec![0.5, -0.3, 0.8, -0.1, 0.0, 0.2, -0.4, 0.6];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = reverb.process(&input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.samples.len(), input.samples.len());
        assert_eq!(output.spec.sample_rate, input.spec.sample_rate);

        // Reverb should produce different output than input
        let mut different_samples = 0;
        for (i, o) in input.samples.iter().zip(output.samples.iter()) {
            if (i - o).abs() > 0.001 {
                different_samples += 1;
            }
        }
        // At least some samples should be different due to the reverb effect
        assert!(different_samples > 0);
    }

    #[test]
    fn test_wet_dry_mix() {
        let mut reverb = ReverbEffect::new();

        // Set to completely dry
        let mut params = Parameters::new();
        params.insert("mix".to_string(), ParameterValue::Float(0.0));
        reverb.set_parameters(params).unwrap();

        let samples = vec![0.5, -0.3, 0.8];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples.clone(), spec);

        let result = reverb.process(&input).unwrap();

        // With completely dry mix, output should be very close to input
        for (i, o) in input.samples.iter().zip(result.samples.iter()) {
            assert!((i - o).abs() < 0.1);
        }
    }

    #[test]
    fn test_parameter_clamping() {
        let mut reverb = ReverbEffect::new();
        let mut params = Parameters::new();
        params.insert("room_size".to_string(), ParameterValue::Float(2.0)); // Above max
        params.insert("feedback".to_string(), ParameterValue::Float(-0.5)); // Below min

        assert!(reverb.set_parameters(params).is_ok());

        let current_params = reverb.get_parameters();
        assert_eq!(
            current_params.get("room_size").unwrap().as_float(),
            Some(1.0)
        ); // Clamped to max
        assert_eq!(
            current_params.get("feedback").unwrap().as_float(),
            Some(0.0)
        ); // Clamped to min
    }

    #[test]
    fn test_reset() {
        let mut reverb = ReverbEffect::new();

        // Process some samples to build up state
        let samples = vec![0.5, -0.3, 0.8, -0.1];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let _result = reverb.process(&input).unwrap();

        // Reset should clear internal state
        reverb.reset();

        // After reset, the low-pass state should be zero
        assert_eq!(reverb.low_pass_state, 0.0);
    }
}
