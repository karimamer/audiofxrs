use std::env;
use hound;
use dasp::signal::{self, Signal};
use dasp::delay::{Delay, DelayLine};

const SAMPLE_RATE: f32 = 44_100.0;
const VIBRATO_RATE: f32 = 5.0;
const VIBRATO_DEPTH: f32 = 0.005; // In seconds

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: vibrato <input_wav> <output_wav>");
        return;
    }
    let input_file = &args[1];
    let output_file = &args[2];

    let mut reader = hound::WavReader::open(input_file).expect("Failed to open input WAV file");
    let spec = reader.spec();
    let mut writer = hound::WavWriter::create(output_file, spec).expect("Failed to create output WAV file");

    let lfo = signal::rate(SAMPLE_RATE as f64).const_hz(VIBRATO_RATE as f64).sine();
    let max_delay_samples = (VIBRATO_DEPTH * SAMPLE_RATE) as usize;
    let mut delay_line = Delay::new(DelayLine::new(max_delay_samples));

    for (sample_result, lfo_value) in reader.samples::<i16>().zip(lfo) {
        let s = sample_result.expect("Failed to read sample");
        let s_f32 = s as f32 / 32_768.0;

        let delay_samples = (0.5 * lfo_value as f32 + 0.5) * max_delay_samples as f32;
        delay_line.set_delay(delay_samples);

        delay_line.tick(s_f32);

        let out_sample = delay_line.output();

        let out_sample_i16 = (out_sample * 32_767.0).max(-32_768.0).min(32_767.0) as i16;
        writer.write_sample(out_sample_i16).expect("Failed to write sample");
    }

    writer.finalize().expect("Failed to finalize WAV writer");
}