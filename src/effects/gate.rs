use crate::audio_io::AudioData;
use crate::effects::dsp::clamp;
use crate::effects::{float_param, AudioEffect, ParameterDef, ParameterValue, Parameters};

pub struct GateEffect {
    sample_rate: f32,

    // Parameters
    threshold: f32,  // Threshold in linear scale (0.0 to 1.0)
    attack_ms: f32,  // Attack time in milliseconds
    hold_ms: f32,    // Hold time in milliseconds
    release_ms: f32, // Release time in milliseconds
    ratio: f32,      // Gate ratio (0.0 to 1.0, 1.0 = full gate)

    // Internal state
    envelope: f32,      // Current envelope level for detection
    gate_state: f32,    // Current gate state (0.0 to 1.0)
    hold_counter: f32,  // Hold time counter in samples
    is_gate_open: bool, // Whether gate is currently open
    attack_coeff: f32,  // Attack coefficient
    release_coeff: f32, // Release coefficient
}

impl Default for GateEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl GateEffect {
    pub fn new() -> Self {
        let mut gate = Self {
            sample_rate: 44100.0,
            threshold: 0.1,
            attack_ms: 1.0,
            hold_ms: 10.0,
            release_ms: 100.0,
            ratio: 1.0,
            envelope: 0.0,
            gate_state: 0.0,
            hold_counter: 0.0,
            is_gate_open: false,
            attack_coeff: 0.0,
            release_coeff: 0.0,
        };

        gate.update_coefficients();
        gate
    }

    fn update_coefficients(&mut self) {
        // Calculate attack and release coefficients
        self.attack_coeff = (-1.0 / (self.attack_ms * 0.001 * self.sample_rate)).exp();
        self.release_coeff = (-1.0 / (self.release_ms * 0.001 * self.sample_rate)).exp();
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        let input_level = input.abs();

        // Simple envelope follower for gate detection
        let env_coeff = if input_level > self.envelope {
            0.99
        } else {
            0.999
        };
        self.envelope = input_level + (self.envelope - input_level) * env_coeff;

        // Determine if gate should be open based on threshold
        let should_open = self.envelope > self.threshold;

        // Gate logic with hysteresis
        if should_open && !self.is_gate_open {
            // Open the gate
            self.is_gate_open = true;
            self.hold_counter = self.hold_ms * 0.001 * self.sample_rate; // Reset hold timer
        } else if !should_open && self.is_gate_open {
            // Start hold period
            self.hold_counter -= 1.0;
            if self.hold_counter <= 0.0 {
                // Hold period expired, close the gate
                self.is_gate_open = false;
            }
        } else if should_open && self.is_gate_open {
            // Signal still above threshold, reset hold timer
            self.hold_counter = self.hold_ms * 0.001 * self.sample_rate;
        }

        // Calculate target gate state
        let target_gate = if self.is_gate_open {
            1.0
        } else {
            1.0 - self.ratio
        };

        // Smooth gate state changes
        let coeff = if target_gate > self.gate_state {
            self.attack_coeff // Fast attack
        } else {
            self.release_coeff // Slower release
        };

        self.gate_state = target_gate + (self.gate_state - target_gate) * coeff;

        // Apply gating to input signal
        let gated = input * self.gate_state;

        // Clamp output
        clamp(gated, -1.0, 1.0)
    }
}

impl AudioEffect for GateEffect {
    fn name(&self) -> &str {
        "Gate"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("threshold", "Gate threshold (0.0 to 1.0)", 0.1, 0.001, 1.0),
            float_param("attack", "Attack time in milliseconds", 1.0, 0.1, 100.0),
            float_param("hold", "Hold time in milliseconds", 10.0, 0.0, 1000.0),
            float_param(
                "release",
                "Release time in milliseconds",
                100.0,
                1.0,
                5000.0,
            ),
            float_param(
                "ratio",
                "Gate ratio (1.0 = full gate, 0.0 = no gate)",
                1.0,
                0.0,
                1.0,
            ),
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
                        .clamp(0.001, 1.0);
                }
                "attack" => {
                    self.attack_ms = value
                        .as_float()
                        .ok_or("Attack parameter must be a number")?
                        .clamp(0.1, 100.0);
                    need_update = true;
                }
                "hold" => {
                    self.hold_ms = value
                        .as_float()
                        .ok_or("Hold parameter must be a number")?
                        .clamp(0.0, 1000.0);
                }
                "release" => {
                    self.release_ms = value
                        .as_float()
                        .ok_or("Release parameter must be a number")?
                        .clamp(1.0, 5000.0);
                    need_update = true;
                }
                "ratio" => {
                    self.ratio = value
                        .as_float()
                        .ok_or("Ratio parameter must be a number")?
                        .clamp(0.0, 1.0);
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
        params.insert("hold".to_string(), ParameterValue::Float(self.hold_ms));
        params.insert(
            "release".to_string(),
            ParameterValue::Float(self.release_ms),
        );
        params.insert("ratio".to_string(), ParameterValue::Float(self.ratio));
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
        self.gate_state = 0.0;
        self.hold_counter = 0.0;
        self.is_gate_open = false;
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
    fn test_gate_creation() {
        let gate = GateEffect::new();
        assert_eq!(gate.name(), "Gate");
        assert_eq!(gate.parameter_definitions().len(), 5);
    }

    #[test]
    fn test_parameter_setting() {
        let mut gate = GateEffect::new();
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(0.2));
        params.insert("attack".to_string(), ParameterValue::Float(2.0));
        params.insert("hold".to_string(), ParameterValue::Float(20.0));
        params.insert("release".to_string(), ParameterValue::Float(200.0));
        params.insert("ratio".to_string(), ParameterValue::Float(0.8));

        assert!(gate.set_parameters(params).is_ok());

        let current_params = gate.get_parameters();
        assert_eq!(
            current_params.get("threshold").unwrap().as_float(),
            Some(0.2)
        );
        assert_eq!(current_params.get("attack").unwrap().as_float(), Some(2.0));
        assert_eq!(current_params.get("hold").unwrap().as_float(), Some(20.0));
        assert_eq!(
            current_params.get("release").unwrap().as_float(),
            Some(200.0)
        );
        assert_eq!(current_params.get("ratio").unwrap().as_float(), Some(0.8));
    }

    #[test]
    fn test_gate_processing() {
        let mut gate = GateEffect::new();

        // Create test audio data with loud and quiet sections
        let samples = vec![0.5, 0.0, 0.0, 0.6, 0.0, 0.0, 0.8, 0.0];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = gate.process(&input);
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
    fn test_gate_blocks_quiet_signals() {
        let mut gate = GateEffect::new();

        // Set a higher threshold
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(0.3));
        params.insert("ratio".to_string(), ParameterValue::Float(1.0)); // Full gate
        gate.set_parameters(params).unwrap();

        // Test with quiet signals that should be gated
        let quiet_sample = 0.1; // Below threshold
        let mut output = 0.0;

        // Process several samples to let the gate settle
        for _ in 0..1000 {
            output = gate.process_sample(quiet_sample);
        }

        // Quiet signals should be heavily attenuated
        assert!(output.abs() < quiet_sample * 0.5);
    }

    #[test]
    fn test_gate_passes_loud_signals() {
        let mut gate = GateEffect::new();

        // Set a low threshold
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(0.05));
        gate.set_parameters(params).unwrap();

        // Test with loud signals that should pass through
        let loud_sample = 0.8; // Above threshold
        let mut output = 0.0;

        // Process several samples to let the gate open
        for _ in 0..1000 {
            output = gate.process_sample(loud_sample);
        }

        // Loud signals should pass through relatively unchanged
        assert!(output.abs() > loud_sample * 0.8);
    }

    #[test]
    fn test_parameter_clamping() {
        let mut gate = GateEffect::new();
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(2.0)); // Above max
        params.insert("attack".to_string(), ParameterValue::Float(0.05)); // Below min
        params.insert("ratio".to_string(), ParameterValue::Float(1.5)); // Above max

        assert!(gate.set_parameters(params).is_ok());

        let current_params = gate.get_parameters();
        assert_eq!(
            current_params.get("threshold").unwrap().as_float(),
            Some(1.0)
        ); // Clamped to max
        assert_eq!(current_params.get("attack").unwrap().as_float(), Some(0.1)); // Clamped to min
        assert_eq!(current_params.get("ratio").unwrap().as_float(), Some(1.0)); // Clamped to max
    }

    #[test]
    fn test_hold_time() {
        let mut gate = GateEffect::new();

        // Set short hold time for testing
        let mut params = Parameters::new();
        params.insert("threshold".to_string(), ParameterValue::Float(0.3));
        params.insert("hold".to_string(), ParameterValue::Float(5.0)); // 5ms hold
        params.insert("attack".to_string(), ParameterValue::Float(1.0)); // Fast attack
        params.insert("release".to_string(), ParameterValue::Float(50.0)); // Medium release
        gate.set_parameters(params).unwrap();

        // Send a loud signal to open the gate
        for _ in 0..100 {
            gate.process_sample(0.5);
        }

        // Now send quiet signals - gate should stay open for hold time
        let mut outputs = Vec::new();
        for _ in 0..500 {
            outputs.push(gate.process_sample(0.01)); // Very quiet
        }

        // The gate should remain somewhat open initially due to hold time
        assert!(outputs[50].abs() > 0.005); // Still some signal getting through
    }

    #[test]
    fn test_reset() {
        let mut gate = GateEffect::new();

        // Process some samples to build up state
        for _ in 0..100 {
            gate.process_sample(0.5);
        }

        // Reset should clear internal state
        gate.reset();
        assert_eq!(gate.envelope, 0.0);
        assert_eq!(gate.gate_state, 0.0);
        assert_eq!(gate.hold_counter, 0.0);
        assert!(!gate.is_gate_open);
    }

    #[test]
    fn test_gate_ratio() {
        let mut gate_full = GateEffect::new();
        let mut gate_partial = GateEffect::new();

        // Set up full gate (ratio = 1.0)
        let mut params1 = Parameters::new();
        params1.insert("threshold".to_string(), ParameterValue::Float(0.3));
        params1.insert("ratio".to_string(), ParameterValue::Float(1.0));
        gate_full.set_parameters(params1).unwrap();

        // Set up partial gate (ratio = 0.5)
        let mut params2 = Parameters::new();
        params2.insert("threshold".to_string(), ParameterValue::Float(0.3));
        params2.insert("ratio".to_string(), ParameterValue::Float(0.5));
        gate_partial.set_parameters(params2).unwrap();

        // Test with quiet signal below threshold
        let quiet_sample = 0.1;
        let mut full_output = 0.0;
        let mut partial_output = 0.0;

        // Process samples to let gates settle
        for _ in 0..1000 {
            full_output = gate_full.process_sample(quiet_sample);
            partial_output = gate_partial.process_sample(quiet_sample);
        }

        // Full gate should attenuate more than partial gate
        assert!(full_output.abs() < partial_output.abs());
        assert!(partial_output.abs() > quiet_sample * 0.3); // Partial gate lets some through
    }

    #[test]
    fn test_attack_and_release_timing() {
        let mut fast_gate = GateEffect::new();
        let mut slow_gate = GateEffect::new();

        // Set up fast attack/release
        let mut params1 = Parameters::new();
        params1.insert("attack".to_string(), ParameterValue::Float(1.0));
        params1.insert("release".to_string(), ParameterValue::Float(10.0));
        params1.insert("threshold".to_string(), ParameterValue::Float(0.3));
        fast_gate.set_parameters(params1).unwrap();

        // Set up slow attack/release
        let mut params2 = Parameters::new();
        params2.insert("attack".to_string(), ParameterValue::Float(20.0));
        params2.insert("release".to_string(), ParameterValue::Float(200.0));
        params2.insert("threshold".to_string(), ParameterValue::Float(0.3));
        slow_gate.set_parameters(params2).unwrap();

        // Test with loud signal
        let loud_samples = vec![0.8; 100];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(loud_samples, spec);

        let fast_result = fast_gate.process(&input).unwrap();
        let slow_result = slow_gate.process(&input).unwrap();

        // Both should process successfully with different timing characteristics
        assert_eq!(fast_result.samples.len(), slow_result.samples.len());

        // Results should be different due to timing differences
        let mut different_samples = 0;
        for (fast, slow) in fast_result.samples.iter().zip(slow_result.samples.iter()) {
            if (fast - slow).abs() > 0.01 {
                different_samples += 1;
            }
        }
        assert!(different_samples > 10); // Should have timing differences
    }
}
