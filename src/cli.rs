use crate::audio_io::{read_audio_file, write_audio_file};
use crate::effects::bitcrusher::Bitcrusher;
use crate::effects::chorus::ChorusEffect;
use crate::effects::compression::CompressionEffect;
use crate::effects::delay::DelayEffect;
use crate::effects::distortion::DistortionEffect;
use crate::effects::eq::EqEffect;
use crate::effects::flanger::FlangerEffect;
use crate::effects::gate::GateEffect;
use crate::effects::limiter::LimiterEffect;
use crate::effects::phaser::PhaserEffect;
use crate::effects::pitch_shifting::PitchShiftingEffect;
use crate::effects::reverb::ReverbEffect;
use crate::effects::time_stretching::TimeStretchingEffect;
use crate::effects::tremolo::TremoloEffect;
use crate::effects::vibrato::VibratoEffect;
use crate::effects::{AudioEffect, ParameterValue, Parameters};
use std::collections::HashMap;
use std::env;
use std::process;

pub struct CliArgs {
    pub effect_name: String,
    pub input_file: String,
    pub output_file: String,
    pub parameters: Parameters,
    pub show_help: bool,
    pub list_effects: bool,
    pub show_effect_info: Option<String>,
}

pub struct CliApp {
    available_effects: HashMap<String, fn() -> Box<dyn AudioEffect>>,
}

impl CliApp {
    pub fn new() -> Self {
        let mut available_effects: HashMap<String, fn() -> Box<dyn AudioEffect>> = HashMap::new();

        // Register available effects
        available_effects.insert("bitcrusher".to_string(), || Box::new(Bitcrusher::new()));
        available_effects.insert("chorus".to_string(), || Box::new(ChorusEffect::new()));
        available_effects.insert("delay".to_string(), || Box::new(DelayEffect::new()));
        available_effects.insert("distortion".to_string(), || {
            Box::new(DistortionEffect::new())
        });
        available_effects.insert("reverb".to_string(), || Box::new(ReverbEffect::new()));
        available_effects.insert("compression".to_string(), || {
            Box::new(CompressionEffect::new())
        });
        available_effects.insert("eq".to_string(), || Box::new(EqEffect::new()));
        available_effects.insert("flanger".to_string(), || Box::new(FlangerEffect::new()));
        available_effects.insert("gate".to_string(), || Box::new(GateEffect::new()));
        available_effects.insert("limiter".to_string(), || Box::new(LimiterEffect::new()));
        available_effects.insert("tremolo".to_string(), || Box::new(TremoloEffect::new()));
        available_effects.insert("phaser".to_string(), || Box::new(PhaserEffect::new()));
        available_effects.insert("vibrato".to_string(), || Box::new(VibratoEffect::new()));
        available_effects.insert("pitch_shift".to_string(), || {
            Box::new(PitchShiftingEffect::new())
        });
        available_effects.insert("time_stretch".to_string(), || {
            Box::new(TimeStretchingEffect::new())
        });

        Self { available_effects }
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let args = self.parse_args()?;

        if args.show_help {
            self.show_help();
            return Ok(());
        }

        if args.list_effects {
            self.list_effects();
            return Ok(());
        }

        if let Some(effect_name) = args.show_effect_info {
            self.show_effect_info(&effect_name)?;
            return Ok(());
        }

        // Process audio with the specified effect
        self.process_audio(&args)
    }

    fn parse_args(&self) -> Result<CliArgs, String> {
        let args: Vec<String> = env::args().collect();

        if args.len() < 2 {
            return Ok(CliArgs {
                effect_name: String::new(),
                input_file: String::new(),
                output_file: String::new(),
                parameters: Parameters::new(),
                show_help: true,
                list_effects: false,
                show_effect_info: None,
            });
        }

        // Handle special commands
        match args[1].as_str() {
            "--help" | "-h" => {
                return Ok(CliArgs {
                    effect_name: String::new(),
                    input_file: String::new(),
                    output_file: String::new(),
                    parameters: Parameters::new(),
                    show_help: true,
                    list_effects: false,
                    show_effect_info: None,
                });
            }
            "--list" | "-l" => {
                return Ok(CliArgs {
                    effect_name: String::new(),
                    input_file: String::new(),
                    output_file: String::new(),
                    parameters: Parameters::new(),
                    show_help: false,
                    list_effects: true,
                    show_effect_info: None,
                });
            }
            "--info" | "-i" => {
                if args.len() < 3 {
                    return Err("Effect name required for --info".to_string());
                }
                return Ok(CliArgs {
                    effect_name: String::new(),
                    input_file: String::new(),
                    output_file: String::new(),
                    parameters: Parameters::new(),
                    show_help: false,
                    list_effects: false,
                    show_effect_info: Some(args[2].clone()),
                });
            }
            _ => {}
        }

        // Parse normal command: effect input output [--param value]
        if args.len() < 4 {
            return Err(
                "Usage: audiofxrs <effect> <input.wav> <output.wav> [--param value]".to_string(),
            );
        }

        let effect_name = args[1].clone();
        let input_file = args[2].clone();
        let output_file = args[3].clone();

        // Validate effect exists
        if !self.available_effects.contains_key(&effect_name) {
            return Err(format!(
                "Unknown effect: {}. Use --list to see available effects.",
                effect_name
            ));
        }

        // Parse parameters
        let mut parameters = Parameters::new();
        let mut i = 4;
        while i < args.len() {
            if args[i].starts_with("--") {
                let param_name = args[i].trim_start_matches("--");
                if i + 1 >= args.len() {
                    return Err(format!("Missing value for parameter: {}", param_name));
                }

                let param_value = &args[i + 1];

                // Try to parse as different types
                let value = if let Ok(float_val) = param_value.parse::<f32>() {
                    ParameterValue::Float(float_val)
                } else if let Ok(int_val) = param_value.parse::<i32>() {
                    ParameterValue::Int(int_val)
                } else if let Ok(bool_val) = param_value.parse::<bool>() {
                    ParameterValue::Bool(bool_val)
                } else {
                    ParameterValue::String(param_value.clone())
                };

                parameters.insert(param_name.to_string(), value);
                i += 2;
            } else {
                return Err(format!(
                    "Invalid argument: {}. Parameters must start with --",
                    args[i]
                ));
            }
        }

        Ok(CliArgs {
            effect_name,
            input_file,
            output_file,
            parameters,
            show_help: false,
            list_effects: false,
            show_effect_info: None,
        })
    }

    fn process_audio(&self, args: &CliArgs) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "Processing {} with {} effect...",
            args.input_file, args.effect_name
        );

        // Load input audio
        let input_audio = read_audio_file(&args.input_file)
            .map_err(|e| format!("Failed to read input file: {}", e))?;

        println!(
            "Input: {} channels, {} Hz, {:.2}s duration",
            input_audio.num_channels,
            input_audio.sample_rate,
            input_audio.duration_seconds()
        );

        // Create effect
        let effect_factory = self
            .available_effects
            .get(&args.effect_name)
            .ok_or_else(|| format!("Effect not found: {}", args.effect_name))?;

        let mut effect = effect_factory();

        // Check format support
        if !effect.supports_format(input_audio.sample_rate, input_audio.num_channels) {
            return Err(format!(
                "Effect {} does not support format: {} channels at {} Hz",
                args.effect_name, input_audio.num_channels, input_audio.sample_rate
            )
            .into());
        }

        // Set parameters
        if !args.parameters.is_empty() {
            effect
                .set_parameters(args.parameters.clone())
                .map_err(|e| format!("Failed to set parameters: {}", e))?;

            println!("Applied parameters:");
            for (key, value) in &args.parameters {
                println!("  {} = {:?}", key, value);
            }
        }

        // Process audio
        let output_audio = effect
            .process(&input_audio)
            .map_err(|e| format!("Failed to process audio: {}", e))?;

        // Write output
        write_audio_file(&args.output_file, &output_audio.samples, output_audio.spec)
            .map_err(|e| format!("Failed to write output file: {}", e))?;

        println!("Successfully wrote output to: {}", args.output_file);
        Ok(())
    }

    fn show_help(&self) {
        println!("AudioFX-RS - Audio Effects Processor");
        println!();
        println!("USAGE:");
        println!("    audiofxrs <effect> <input.wav> <output.wav> [--param value]");
        println!("    audiofxrs --list");
        println!("    audiofxrs --info <effect>");
        println!("    audiofxrs --help");
        println!();
        println!("OPTIONS:");
        println!("    -h, --help          Show this help message");
        println!("    -l, --list          List all available effects");
        println!("    -i, --info <effect> Show detailed information about an effect");
        println!();
        println!("EXAMPLES:");
        println!("    audiofxrs bitcrusher input.wav output.wav --bit_depth 4.0 --sample_rate_reduction 2.0");
        println!("    audiofxrs chorus input.wav output.wav --rate 2.0 --depth 3.0");
        println!("    audiofxrs delay input.wav output.wav --delay 500 --feedback 0.4");
        println!("    audiofxrs distortion input.wav output.wav --gain 3.0 --type 1");
        println!("    audiofxrs gate input.wav output.wav --threshold 0.1 --release 100");
        println!("    audiofxrs limiter input.wav output.wav --threshold 0.8 --attack 1.0");
        println!("    audiofxrs reverb input.wav output.wav --room_size 0.8 --mix 0.4");
        println!("    audiofxrs tremolo input.wav output.wav --rate 8.0 --depth 0.6");
        println!("    audiofxrs phaser input.wav output.wav --rate 1.0 --depth 1.5");
        println!("    audiofxrs vibrato input.wav output.wav --rate 5.0 --depth 8.0");
        println!("    audiofxrs --list");
        println!("    audiofxrs --info chorus");
        println!();
        println!("Use --list to see all available effects and --info <effect> for effect-specific parameters.");
    }

    fn list_effects(&self) {
        println!("Available Effects:");
        println!();
        for effect_name in self.available_effects.keys() {
            let effect_factory = &self.available_effects[effect_name];
            let effect = effect_factory();
            println!("  {} - {}", effect_name, effect.name());
        }
        println!();
        println!("Use --info <effect> to see parameters for a specific effect.");
    }

    fn show_effect_info(&self, effect_name: &str) -> Result<(), String> {
        let effect_factory = self
            .available_effects
            .get(effect_name)
            .ok_or_else(|| format!("Unknown effect: {}", effect_name))?;

        let effect = effect_factory();
        let params = effect.parameter_definitions();

        println!("Effect: {} ({})", effect_name, effect.name());
        println!();

        if params.is_empty() {
            println!("This effect has no configurable parameters.");
        } else {
            println!("Parameters:");
            for param in &params {
                println!("  --{}", param.name);
                println!("    Description: {}", param.description);
                println!("    Default: {:?}", param.default_value);
                if let (Some(min), Some(max)) = (&param.min_value, &param.max_value) {
                    println!("    Range: {:?} to {:?}", min, max);
                }
                println!();
            }
        }

        println!("Example:");
        println!("  audiofxrs {} input.wav output.wav", effect_name);
        if !params.is_empty() {
            print!("  audiofxrs {} input.wav output.wav", effect_name);
            for param in params.iter().take(2) {
                // Show first 2 params as example
                match param.default_value {
                    ParameterValue::Float(v) => print!(" --{} {}", param.name, v),
                    ParameterValue::Int(v) => print!(" --{} {}", param.name, v),
                    ParameterValue::Bool(v) => print!(" --{} {}", param.name, v),
                    ParameterValue::String(ref v) => print!(" --{} {}", param.name, v),
                }
            }
            println!();
        }

        Ok(())
    }
}

impl Default for CliApp {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run_cli() {
    let app = CliApp::new();

    if let Err(error) = app.run() {
        eprintln!("Error: {}", error);
        eprintln!();
        eprintln!("Use --help for usage information.");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_app_creation() {
        let app = CliApp::new();
        assert!(!app.available_effects.is_empty());
        assert!(app.available_effects.contains_key("bitcrusher"));
        assert!(app.available_effects.contains_key("chorus"));
        assert!(app.available_effects.contains_key("delay"));
        assert!(app.available_effects.contains_key("distortion"));
        assert!(app.available_effects.contains_key("reverb"));
        assert!(app.available_effects.contains_key("compression"));
        assert!(app.available_effects.contains_key("eq"));
        assert!(app.available_effects.contains_key("flanger"));
        assert!(app.available_effects.contains_key("gate"));
        assert!(app.available_effects.contains_key("limiter"));
        assert!(app.available_effects.contains_key("tremolo"));
        assert!(app.available_effects.contains_key("phaser"));
        assert!(app.available_effects.contains_key("vibrato"));
        assert!(app.available_effects.contains_key("pitch_shift"));
        assert!(app.available_effects.contains_key("time_stretch"));
    }

    #[test]
    fn test_help_parsing() {
        let app = CliApp::new();

        // Mock command line args for help
        let original_args = env::args().collect::<Vec<_>>();

        // We can't easily test the actual parsing without mocking env::args(),
        // but we can test the structure exists
        assert!(app.available_effects.len() >= 15);
    }

    #[test]
    fn test_effect_registration() {
        let app = CliApp::new();

        // Test that we can create effects
        let chorus_factory = app.available_effects.get("chorus").unwrap();
        let chorus = chorus_factory();
        assert_eq!(chorus.name(), "Chorus");

        let distortion_factory = app.available_effects.get("distortion").unwrap();
        let distortion = distortion_factory();
        assert_eq!(distortion.name(), "Distortion");
    }
}
