/*
Pitch shifting is a process that alters the pitch of an audio signal without changing its duration.
One approach to pitch shifting is to use granular synthesis, which involves dividing the input signal into small chunks or grains, and then processing and reassembling these grains to achieve the desired pitch shift.
 */
use std::env;
use hound;
use dasp::signal::{self, Signal};
use dasp::interpolate::linear::Linear;

const SAMPLE_RATE: f64 = 44_100.0;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        println!("Usage: pitch_shift <input_wav> <output_wav> <pitch_shift_factor>");
        return;
    }
    let input_file = &args[1];
    let output_file = &args[2];
    let pitch_shift_factor: f64 = args[3].parse().expect("Invalid pitch shift factor");

    let mut reader = hound::WavReader::open(input_file).expect("Failed to open input WAV file");
    let spec = reader.spec();
    let num_samples = reader.len() as usize;
    let mut writer = hound::WavWriter::create(output_file, spec).expect("Failed to create output WAV file");

    let samples: Vec<f32> = reader.samples::<i16>()
        .map(|s| s.expect("Failed to read sample") as f32 / 32_768.0)
        .collect();

    let grain_size = 512;
    let grain_overlap = 4;
    let step_size = grain_size / grain_overlap;

    let mut output_samples = Vec::new();

    for grain_start in (0..num_samples).step_by(step_size) {
        let grain_end = grain_start + grain_size;
        if grain_end >= num_samples {
            break;
        }

        let grain: Vec<f32> = samples[grain_start..grain_end].to_vec();
        let windowed_grain: Vec<f32> = grain.iter().enumerate().map(|(i, s)| {
            let window = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (grain_size - 1)).cos();
            s * window as f32
        }).collect();

        let interpolated_signal = signal::from_interleaved_samples_iter(windowed_grain.into_iter())
            .from_hz(SAMPLE_RATE)
            .interpolate::<Linear>()
            .step_ratio(1.0 / pitch_shift_factor);

        let pitch_shifted_grain: Vec<f32> = interpolated_signal.take(grain_size).map(|f| f[0]).collect();
        output_samples.extend(pitch_shifted_grain);
    }

    for sample in output_samples.iter() {
        let out_sample_i16 = (sample * 32_767.0).max(-32_768.0).min(32_767.0) as i16;
        writer.write_sample(out_sample_i16).expect("Failed to write sample");
    }

    writer.finalize().expect("Failed to finalize WAV writer");
}