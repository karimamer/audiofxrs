/*
A tremolo effect modulates the amplitude of the audio signal using a low-frequency oscillator (LFO). 
This creates a pulsating effect in the output audio.
This program uses an LFO with a sine waveform to modulate the amplitude of the input signal. The TREMOLO_RATE constant controls the speed of the tremolo effect, and the TREMOLO_DEPTH constant controls the depth (intensity) of the effect. The LFO output is scaled and shifted to be in the range [0, 1], and then multiplied by the input signal to produce the output.
 */
 use std::env;
 use hound;
 use dasp::signal::{self, Signal};
 
 const SAMPLE_RATE: f32 = 44_100.0;
 const TREMOLO_RATE: f32 = 5.0;
 const TREMOLO_DEPTH: f32 = 0.7;
 
 fn main() {
     let args: Vec<String> = env::args().collect();
     if args.len() != 3 {
         println!("Usage: tremolo <input_wav> <output_wav>");
         return;
     }
     let input_file = &args[1];
     let output_file = &args[2];
 
     let mut reader = hound::WavReader::open(input_file).expect("Failed to open input WAV file");
     let spec = reader.spec();
     let mut writer = hound::WavWriter::create(output_file, spec).expect("Failed to create output WAV file");
 
     let lfo = signal::rate(SAMPLE_RATE as f64).const_hz(TREMOLO_RATE as f64).sine();
 
     for (sample_result, lfo_value) in reader.samples::<i16>().zip(lfo) {
         let s = sample_result.expect("Failed to read sample");
         let s_f32 = s as f32 / 32_768.0;
 
         let gain = 1.0 - TREMOLO_DEPTH * (0.5 * lfo_value as f32 + 0.5);
         let out_sample = s_f32 * gain;
 
         let out_sample_i16 = (out_sample * 32_767.0).max(-32_768.0).min(32_767.0) as i16;
         writer.write_sample(out_sample_i16).expect("Failed to write sample");
     }
 
     writer.finalize().expect("Failed to finalize WAV writer");
 }