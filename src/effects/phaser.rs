use crate::audio_io::AudioData;
use crate::effects::dsp::{clamp, sine_wave};
use crate::effects::{float_param, AudioEffect, ParameterDef, ParameterValue, Parameters};

pub struct PhaserEffect {
    sample_rate: f32,
    phase: f32,

    // Parameters
    rate_hz: f32,
    depth: f32,
    feedback: f32,
    wet_dry_mix: f32,

    // Internal state - simplified all-pass filter chain
    all_pass_states: Vec<f32>,
}

impl Default for PhaserEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl PhaserEffect {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            phase: 0.0,
            rate_hz: 0.5,
            depth: 1.0,
            feedback: 0.7,
            wet_dry_mix: 0.5,
            all_pass_states: vec![0.0; 4], // 4-stage all-pass filter
        }
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        // Generate LFO
        let lfo = sine_wave(self.phase);
        self.phase += self.rate_hz / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Calculate modulated phase shift
        let phase_shift = self.depth * lfo;

        // Simple all-pass filter chain approximation
        let mut signal = input;
        for (i, state) in self.all_pass_states.iter_mut().enumerate() {
            let delay_factor = 0.5 + 0.3 * phase_shift * (i as f32 + 1.0) / 4.0;
            let delayed = *state * delay_factor;
            let output = signal + delayed;
            *state = signal - delayed * 0.7;
            signal = output;
        }

        // Apply feedback
        let phased_signal = signal + input * self.feedback * 0.3;

        // Mix wet and dry signals
        let output = input * (1.0 - self.wet_dry_mix) + phased_signal * self.wet_dry_mix;

        clamp(output, -1.0, 1.0)
    }
}

impl AudioEffect for PhaserEffect {
    fn name(&self) -> &str {
        "Phaser"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("rate", "LFO rate in Hz", 0.5, 0.1, 10.0),
            float_param("depth", "Modulation depth", 1.0, 0.0, 2.0),
            float_param("feedback", "Feedback amount", 0.7, 0.0, 0.9),
            float_param("mix", "Wet/dry mix", 0.5, 0.0, 1.0),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        for (key, value) in params {
            match key.as_str() {
                "rate" => {
                    self.rate_hz = value
                        .as_float()
                        .ok_or("Rate parameter must be a number")?
                        .clamp(0.1, 10.0);
                }
                "depth" => {
                    self.depth = value
                        .as_float()
                        .ok_or("Depth parameter must be a number")?
                        .clamp(0.0, 2.0);
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
                _ => return Err(format!("Unknown parameter: {}", key)),
            }
        }
        Ok(())
    }

    fn get_parameters(&self) -> Parameters {
        let mut params = Parameters::new();
        params.insert("rate".to_string(), ParameterValue::Float(self.rate_hz));
        params.insert("depth".to_string(), ParameterValue::Float(self.depth));
        params.insert("feedback".to_string(), ParameterValue::Float(self.feedback));
        params.insert("mix".to_string(), ParameterValue::Float(self.wet_dry_mix));
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
        self.phase = 0.0;
        self.all_pass_states.fill(0.0);
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
    fn test_phaser_creation() {
        let phaser = PhaserEffect::new();
        assert_eq!(phaser.name(), "Phaser");
        assert_eq!(phaser.parameter_definitions().len(), 4);
    }

    #[test]
    fn test_parameter_setting() {
        let mut phaser = PhaserEffect::new();
        let mut params = Parameters::new();
        params.insert("rate".to_string(), ParameterValue::Float(1.0));
        params.insert("depth".to_string(), ParameterValue::Float(1.5));
        params.insert("feedback".to_string(), ParameterValue::Float(0.8));

        assert!(phaser.set_parameters(params).is_ok());

        let current_params = phaser.get_parameters();
        assert_eq!(current_params.get("rate").unwrap().as_float(), Some(1.0));
        assert_eq!(current_params.get("depth").unwrap().as_float(), Some(1.5));
        assert_eq!(
            current_params.get("feedback").unwrap().as_float(),
            Some(0.8)
        );
    }

    #[test]
    fn test_phaser_processing() {
        let mut phaser = PhaserEffect::new();

        // Create test audio data
        let samples = vec![0.5, -0.3, 0.8, -0.1, 0.0, 0.2];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = phaser.process(&input);
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
    fn test_parameter_clamping() {
        let mut phaser = PhaserEffect::new();
        let mut params = Parameters::new();
        params.insert("rate".to_string(), ParameterValue::Float(100.0)); // Above max
        params.insert("feedback".to_string(), ParameterValue::Float(-0.5)); // Below min

        assert!(phaser.set_parameters(params).is_ok());

        let current_params = phaser.get_parameters();
        assert_eq!(current_params.get("rate").unwrap().as_float(), Some(10.0)); // Clamped to max
        assert_eq!(
            current_params.get("feedback").unwrap().as_float(),
            Some(0.0)
        ); // Clamped to min
    }

    #[test]
    fn test_reset() {
        let mut phaser = PhaserEffect::new();

        // Process some samples to build up state
        let samples = vec![0.5, -0.3, 0.8];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let _result = phaser.process(&input).unwrap();

        // Reset should clear internal state
        phaser.reset();
        assert_eq!(phaser.phase, 0.0);
        assert!(phaser.all_pass_states.iter().all(|&x| x == 0.0));
    }
}
