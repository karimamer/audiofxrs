use super::{AudioData, AudioEffect, ParameterDef, ParameterValue, Parameters, float_param};
use std::f32::consts::PI;

pub struct AutoWahEffect {
    sample_rate: f32,
    
    // Auto-wah parameters
    sensitivity: f32,
    frequency_range: f32,
    base_frequency: f32,
    resonance: f32,
    attack_time: f32,
    release_time: f32,
    
    // Internal state
    envelope: f32,
    attack_coeff: f32,
    release_coeff: f32,
    
    // Biquad filter state
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    
    // Filter coefficients
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl AutoWahEffect {
    pub fn new() -> Self {
        let mut effect = Self {
            sample_rate: 44100.0,
            sensitivity: 0.5,
            frequency_range: 1000.0,
            base_frequency: 200.0,
            resonance: 2.0,
            attack_time: 10.0,
            release_time: 100.0,
            envelope: 0.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        };
        
        effect.update_envelope_coefficients();
        effect.update_filter_coefficients(effect.base_frequency);
        effect
    }

    fn update_envelope_coefficients(&mut self) {
        // Convert milliseconds to samples and calculate exponential coefficients
        let attack_samples = (self.attack_time / 1000.0) * self.sample_rate;
        let release_samples = (self.release_time / 1000.0) * self.sample_rate;
        
        self.attack_coeff = if attack_samples > 0.0 {
            (-1.0 / attack_samples).exp()
        } else {
            0.0
        };
        
        self.release_coeff = if release_samples > 0.0 {
            (-1.0 / release_samples).exp()
        } else {
            0.0
        };
    }

    fn update_filter_coefficients(&mut self, frequency: f32) {
        // Resonant bandpass filter (peak EQ style)
        let freq = frequency.clamp(20.0, self.sample_rate * 0.45);
        let omega = 2.0 * PI * freq / self.sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let q = self.resonance.clamp(0.1, 20.0);
        let alpha = sin_omega / (2.0 * q);

        // Bandpass filter coefficients
        let norm = 1.0 + alpha;
        self.b0 = alpha / norm;
        self.b1 = 0.0;
        self.b2 = -alpha / norm;
        self.a1 = -2.0 * cos_omega / norm;
        self.a2 = (1.0 - alpha) / norm;
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        // Envelope follower
        let input_level = input.abs();
        
        if input_level > self.envelope {
            // Attack
            self.envelope = input_level + (self.envelope - input_level) * self.attack_coeff;
        } else {
            // Release
            self.envelope = input_level + (self.envelope - input_level) * self.release_coeff;
        }

        // Map envelope to filter frequency
        let envelope_scaled = (self.envelope * self.sensitivity).min(1.0);
        let target_frequency = self.base_frequency + (envelope_scaled * self.frequency_range);
        
        // Update filter coefficients
        self.update_filter_coefficients(target_frequency);

        // Apply biquad filter
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2 
                   - self.a1 * self.y1 - self.a2 * self.y2;

        // Update delay lines
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        // Mix with dry signal for more musical result
        let dry_mix = 0.3;
        let wet_mix = 0.7;
        dry_mix * input + wet_mix * output
    }
}

impl Default for AutoWahEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEffect for AutoWahEffect {
    fn name(&self) -> &str {
        "Auto-Wah"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("sensitivity", "Envelope sensitivity (0.0-2.0)", 0.5, 0.0, 2.0),
            float_param("frequency_range", "Filter frequency range in Hz (100-3000)", 1000.0, 100.0, 3000.0),
            float_param("base_frequency", "Base filter frequency in Hz (50-800)", 200.0, 50.0, 800.0),
            float_param("resonance", "Filter resonance/Q factor (0.1-10.0)", 2.0, 0.1, 10.0),
            float_param("attack_time", "Envelope attack time in ms (1.0-100.0)", 10.0, 1.0, 100.0),
            float_param("release_time", "Envelope release time in ms (10.0-1000.0)", 100.0, 10.0, 1000.0),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        for (key, value) in params {
            match key.as_str() {
                "sensitivity" => {
                    self.sensitivity = value.as_float()
                        .ok_or("Sensitivity must be a float")?
                        .clamp(0.0, 2.0);
                }
                "frequency_range" => {
                    self.frequency_range = value.as_float()
                        .ok_or("Frequency range must be a float")?
                        .clamp(100.0, 3000.0);
                }
                "base_frequency" => {
                    self.base_frequency = value.as_float()
                        .ok_or("Base frequency must be a float")?
                        .clamp(50.0, 800.0);
                }
                "resonance" => {
                    self.resonance = value.as_float()
                        .ok_or("Resonance must be a float")?
                        .clamp(0.1, 10.0);
                }
                "attack_time" => {
                    let new_attack = value.as_float()
                        .ok_or("Attack time must be a float")?
                        .clamp(1.0, 100.0);
                    if (new_attack - self.attack_time).abs() > f32::EPSILON {
                        self.attack_time = new_attack;
                        self.update_envelope_coefficients();
                    }
                }
                "release_time" => {
                    let new_release = value.as_float()
                        .ok_or("Release time must be a float")?
                        .clamp(10.0, 1000.0);
                    if (new_release - self.release_time).abs() > f32::EPSILON {
                        self.release_time = new_release;
                        self.update_envelope_coefficients();
                    }
                }
                _ => return Err(format!("Unknown parameter: {}", key)),
            }
        }
        Ok(())
    }

    fn get_parameters(&self) -> Parameters {
        let mut params = Parameters::new();
        params.insert("sensitivity".to_string(), ParameterValue::Float(self.sensitivity));
        params.insert("frequency_range".to_string(), ParameterValue::Float(self.frequency_range));
        params.insert("base_frequency".to_string(), ParameterValue::Float(self.base_frequency));
        params.insert("resonance".to_string(), ParameterValue::Float(self.resonance));
        params.insert("attack_time".to_string(), ParameterValue::Float(self.attack_time));
        params.insert("release_time".to_string(), ParameterValue::Float(self.release_time));
        params
    }

    fn process(&mut self, input: &AudioData) -> Result<AudioData, String> {
        if self.sample_rate != input.sample_rate as f32 {
            self.sample_rate = input.sample_rate as f32;
            self.update_envelope_coefficients();
        }

        let mut output_samples = Vec::with_capacity(input.samples.len());
        
        for &sample in &input.samples {
            let processed = self.process_sample(sample);
            output_samples.push(processed);
        }

        Ok(AudioData::new(output_samples, input.spec))
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    fn supports_format(&self, sample_rate: u32, channels: usize) -> bool {
        sample_rate >= 8000 && sample_rate <= 192_000 && channels >= 1 && channels <= 8
    }
}