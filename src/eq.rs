use std::env;
use hound;
use dasp::{Frame, ring_buffer};
use dasp::frame::Mono;
use dasp::signal::{self, Signal};
use dasp::interpolate::linear::Linear;
use dasp::filter::{Biquad, LowShelf, Peaking, HighShelf};

const SAMPLE_RATE: f64 = 44_100.0;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: equalizer <input_wav> <output_wav>");
        return;
    }
    let input_file = &args[1];
    let output_file = &args[2];

    let mut reader = hound::WavReader::open(input_file).expect("Failed to open input WAV file");
    let spec = reader.spec();
    let mut writer = hound::WavWriter::create(output_file, spec).expect("Failed to create output WAV file");

    // Set up equalizer bands
    let low_freq = 100.0;
    let mid_freq = 1000.0;
    let high_freq = 5000.0;

    let low_gain = 3.0; // In decibels
    let mid_gain = -2.0;
    let high_gain = 4.0;

    let q = 1.0; // Q factor for the peaking filter

    let low_shelf_filter = Biquad::from_low_shelf(SAMPLE_RATE, low_freq, low_gain);
    let peaking_filter = Biquad::from_peaking(SAMPLE_RATE, mid_freq, q, mid_gain);
    let high_shelf_filter = Biquad::from_high_shelf(SAMPLE_RATE, high_freq, high_gain);

    let mut low_shelf = signal::from_fn(move |_| low_shelf_filter.next().unwrap_or_default());
    let mut peaking = signal::from_fn(move |_| peaking_filter.next().unwrap_or_default());
    let mut high_shelf = signal::from_fn(move |_| high_shelf_filter.next().unwrap_or_default());

    for sample_result in reader.samples::<i16>() {
        let s = sample_result.expect("Failed to read sample");
        let s_f32 = s as f32 / 32_768.0;
        let s_frame = Mono(s_f32);

        // Apply filters
        let low_eq = low_shelf.eq(s_frame);
        let mid_eq = peaking.eq(low_eq);
        let high_eq = high_shelf.eq(mid_eq);

        let out_sample = high_eq[0];
        let out_sample_i16 = (out_sample * 32_767.0).max(-32_768.0).min(32_767.0) as i16;
        writer.write_sample(out_sample_i16).expect("Failed to write sample");
    }

    writer.finalize().expect("Failed to finalize WAV writer");
}