/*
A phaser is an audio effect that creates a sweeping sound by adding the original audio signal with a filtered and phase-shifted version of itself. 
The phase shift is continuously varied using a low-frequency oscillator (LFO), which creates a series of notches in the frequency spectrum that move over time. 
The result is a distinctive, spacey sound.
This program uses a chain of all-pass filters to create the phase-shifted version of the input signal. 
The center frequencies of the all-pass filters are modulated by an LFO, creating the moving notches characteristic of a phaser.
The filtered signal is then added to the original signal to produce the output.
 */

use std::env;
use hound;
use dasp::signal::{self, Signal};
use biquad::{Biquad, DirectForm2, Params};

const SAMPLE_RATE: f32 = 44_100.0;
const PHASER_DEPTH: f32 = 1.0;
const PHASER_RATE: f32 = 0.5;
const PHASER_FEEDBACK: f32 = 0.7;
const NUM_ALL_PASS_FILTERS: usize = 4;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: phaser <input_wav> <output_wav>");
        return;
    }
    let input_file = &args[1];
    let output_file = &args[2];

    let mut reader = hound::WavReader::open(input_file).expect("Failed to open input WAV file");
    let spec = reader.spec();
    let mut writer = hound::WavWriter::create(output_file, spec).expect("Failed to create output WAV file");

    let lfo = signal::rate(SAMPLE_RATE as f64).const_hz(PHASER_RATE as f64).sine();
    let mut all_pass_filters: Vec<Biquad<DirectForm2>> = vec![Biquad::default(); NUM_ALL_PASS_FILTERS];
    let mut feedback_sample = 0.0;

    for (sample_result, lfo_value) in reader.samples::<i16>().zip(lfo) {
        let s = sample_result.expect("Failed to read sample");
        let s_f32 = s as f32 / 32_768.0;

        let modulated_phase_shift = PHASER_DEPTH * lfo_value as f32;
        let input_sample = s_f32 + PHASER_FEEDBACK * feedback_sample;

        let mut filtered_sample = input_sample;
        for apf in &mut all_pass_filters {
            let q_value = 1.0;
            let params = Params::allpass(SAMPLE_RATE, modulated_phase_shift, q_value);
            apf.update_coefficients(&params);
            filtered_sample = apf.process(filtered_sample);
        }

        let out_sample = s_f32 + filtered_sample;
        feedback_sample = filtered_sample;

        let out_sample_i16 = (out_sample * 32_767.0).max(-32_768.0).min(32_767.0) as i16;
        writer.write_sample(out_sample_i16).expect("Failed to write sample");
    }

    writer.finalize().expect("Failed to finalize WAV writer");
}