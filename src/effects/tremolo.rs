use crate::audio_io::AudioData;
use crate::effects::dsp::sine_wave;
use crate::effects::{
    float_param, int_param, AudioEffect, ParameterDef, ParameterValue, Parameters,
};

#[derive(Debug, Clone, Copy)]
pub enum WaveShape {
    Sine,
    Triangle,
    Square,
    Sawtooth,
}

impl WaveShape {
    fn from_int(value: i32) -> Self {
        match value {
            0 => WaveShape::Sine,
            1 => WaveShape::Triangle,
            2 => WaveShape::Square,
            3 => WaveShape::Sawtooth,
            _ => WaveShape::Sine,
        }
    }

    fn to_int(self) -> i32 {
        match self {
            WaveShape::Sine => 0,
            WaveShape::Triangle => 1,
            WaveShape::Square => 2,
            WaveShape::Sawtooth => 3,
        }
    }
}

pub struct TremoloEffect {
    sample_rate: f32,
    phase: f32,

    // Parameters
    rate_hz: f32,
    depth: f32,
    wave_shape: WaveShape,
}

impl Default for TremoloEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl TremoloEffect {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            phase: 0.0,
            rate_hz: 5.0,
            depth: 0.7,
            wave_shape: WaveShape::Sine,
        }
    }

    fn generate_lfo(&self, phase: f32) -> f32 {
        match self.wave_shape {
            WaveShape::Sine => sine_wave(phase),
            WaveShape::Triangle => {
                let t = phase - phase.floor();
                if t < 0.5 {
                    4.0 * t - 1.0
                } else {
                    3.0 - 4.0 * t
                }
            }
            WaveShape::Square => {
                let t = phase - phase.floor();
                if t < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            WaveShape::Sawtooth => {
                let t = phase - phase.floor();
                2.0 * t - 1.0
            }
        }
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        // Generate LFO
        let lfo = self.generate_lfo(self.phase);

        // Update phase
        self.phase += self.rate_hz / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Calculate gain modulation
        // Scale LFO from [-1, 1] to [1-depth, 1]
        let gain = 1.0 - self.depth * (0.5 * lfo + 0.5);

        // Apply tremolo
        input * gain
    }
}

impl AudioEffect for TremoloEffect {
    fn name(&self) -> &str {
        "Tremolo"
    }

    fn parameter_definitions(&self) -> Vec<ParameterDef> {
        vec![
            float_param("rate", "Tremolo rate in Hz", 5.0, 0.1, 20.0),
            float_param("depth", "Modulation depth (0.0 to 1.0)", 0.7, 0.0, 1.0),
            int_param(
                "wave",
                "Wave shape (0=Sine, 1=Triangle, 2=Square, 3=Sawtooth)",
                0,
                0,
                3,
            ),
        ]
    }

    fn set_parameters(&mut self, params: Parameters) -> Result<(), String> {
        for (key, value) in params {
            match key.as_str() {
                "rate" => {
                    self.rate_hz = value
                        .as_float()
                        .ok_or("Rate parameter must be a number")?
                        .clamp(0.1, 20.0);
                }
                "depth" => {
                    self.depth = value
                        .as_float()
                        .ok_or("Depth parameter must be a number")?
                        .clamp(0.0, 1.0);
                }
                "wave" => {
                    let wave_int = value
                        .as_int()
                        .ok_or("Wave parameter must be an integer")?
                        .clamp(0, 3);
                    self.wave_shape = WaveShape::from_int(wave_int);
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
        params.insert(
            "wave".to_string(),
            ParameterValue::Int(self.wave_shape.to_int()),
        );
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
    fn test_tremolo_creation() {
        let tremolo = TremoloEffect::new();
        assert_eq!(tremolo.name(), "Tremolo");
        assert_eq!(tremolo.parameter_definitions().len(), 3);
    }

    #[test]
    fn test_parameter_setting() {
        let mut tremolo = TremoloEffect::new();
        let mut params = Parameters::new();
        params.insert("rate".to_string(), ParameterValue::Float(10.0));
        params.insert("depth".to_string(), ParameterValue::Float(0.9));
        params.insert("wave".to_string(), ParameterValue::Int(1));

        assert!(tremolo.set_parameters(params).is_ok());

        let current_params = tremolo.get_parameters();
        assert_eq!(current_params.get("rate").unwrap().as_float(), Some(10.0));
        assert_eq!(current_params.get("depth").unwrap().as_float(), Some(0.9));
        assert_eq!(current_params.get("wave").unwrap().as_int(), Some(1));
    }

    #[test]
    fn test_wave_shape_conversion() {
        assert_eq!(WaveShape::from_int(0).to_int(), 0);
        assert_eq!(WaveShape::from_int(3).to_int(), 3);
        assert_eq!(WaveShape::from_int(99).to_int(), 0); // Should default to Sine
    }

    #[test]
    fn test_tremolo_processing() {
        let mut tremolo = TremoloEffect::new();

        // Create test audio data
        let samples = vec![0.5, -0.3, 0.8, -0.1, 0.0, 0.2];
        let spec = default_wav_spec(1, 44100);
        let input = AudioData::new(samples, spec);

        let result = tremolo.process(&input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.samples.len(), input.samples.len());
        assert_eq!(output.spec.sample_rate, input.spec.sample_rate);
    }

    #[test]
    fn test_tremolo_modulation() {
        let mut tremolo = TremoloEffect::new();

        // Set high depth for noticeable effect
        let mut params = Parameters::new();
        params.insert("depth".to_string(), ParameterValue::Float(1.0));
        params.insert("rate".to_string(), ParameterValue::Float(1.0));
        tremolo.set_parameters(params).unwrap();

        // Test with a constant input
        let input_sample = 0.5;
        let mut outputs = Vec::new();

        // Process several samples to see modulation
        for _ in 0..100 {
            outputs.push(tremolo.process_sample(input_sample));
        }

        // The output should vary due to tremolo modulation
        let min_output = outputs.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_output = outputs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        // There should be variation in the output
        assert!(max_output - min_output > 0.1);
    }

    #[test]
    fn test_different_wave_shapes() {
        let mut tremolo = TremoloEffect::new();

        let test_cases = vec![
            (WaveShape::Sine, 0),
            (WaveShape::Triangle, 1),
            (WaveShape::Square, 2),
            (WaveShape::Sawtooth, 3),
        ];

        for (shape, shape_int) in test_cases {
            let mut params = Parameters::new();
            params.insert("wave".to_string(), ParameterValue::Int(shape_int));
            tremolo.set_parameters(params).unwrap();

            // Generate a few LFO samples
            let lfo1 = tremolo.generate_lfo(0.0);
            let lfo2 = tremolo.generate_lfo(0.25);
            let lfo3 = tremolo.generate_lfo(0.5);

            // Each wave shape should produce different patterns
            match shape {
                WaveShape::Sine => {
                    assert!((lfo1 - 0.0).abs() < 0.001); // sin(0) = 0
                    assert!(lfo2 > 0.5); // sin(π/2) ≈ 1
                }
                WaveShape::Square => {
                    assert_eq!(lfo1, 1.0); // First half of square wave
                    assert_eq!(lfo3, -1.0); // Second half of square wave
                }
                _ => {
                    // Just ensure they're in valid range
                    assert!(lfo1 >= -1.0 && lfo1 <= 1.0);
                    assert!(lfo2 >= -1.0 && lfo2 <= 1.0);
                    assert!(lfo3 >= -1.0 && lfo3 <= 1.0);
                }
            }
        }
    }

    #[test]
    fn test_parameter_clamping() {
        let mut tremolo = TremoloEffect::new();
        let mut params = Parameters::new();
        params.insert("rate".to_string(), ParameterValue::Float(100.0)); // Above max
        params.insert("depth".to_string(), ParameterValue::Float(-0.5)); // Below min

        assert!(tremolo.set_parameters(params).is_ok());

        let current_params = tremolo.get_parameters();
        assert_eq!(current_params.get("rate").unwrap().as_float(), Some(20.0)); // Clamped to max
        assert_eq!(current_params.get("depth").unwrap().as_float(), Some(0.0)); // Clamped to min
    }
}
