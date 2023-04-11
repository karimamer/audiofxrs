use std::env;
use hound;

const SAMPLE_RATE: u32 = 44100;
const FLANGER_DEPTH: f32 = 0.002;
const FLANGER_RATE: f32 = 0.5;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: flanger <input_wav> <output_wav>");
        return;
    }
    let input_file = &args[1];
    let output_file = &args[2];

    let mut reader = hound::WavReader::open(input_file).expect("Failed to open input WAV file");
    let spec = reader.spec();
    let mut writer = hound::WavWriter::create(output_file, spec).expect("Failed to create output WAV file");

    let delay_line_len = (SAMPLE_RATE as f32 * FLANGER_DEPTH) as usize;
    let mut delay_line = vec![0.0; delay_line_len];
    let mut write_head = 0;
    let mut read_head = 0;
    let mut sample_counter: u32 = 0;

    for result in reader.samples::<i16>() {
        let s = result.expect("Failed to read sample");
        let s_f32 = s as f32;

        let lfo = (2.0 * std::f32::consts::PI * FLANGER_RATE * sample_counter as f32 / SAMPLE_RATE as f32).sin();
        let modulated_delay_time = FLANGER_DEPTH * lfo;
        let modulated_delay_samples = (modulated_delay_time * SAMPLE_RATE as f32) as isize;

        read_head = (write_head as isize - modulated_delay_samples).rem_euclid(delay_line_len as isize) as usize;
        let delayed_sample = delay_line[read_head];
        let out_sample = s_f32 + delayed_sample;

        delay_line[write_head] = s_f32;
        write_head = (write_head + 1) % delay_line_len;

        let out_sample_i16 = (out_sample.max(-32768.0).min(32767.0)) as i16;
        writer.write_sample(out_sample_i16).expect("Failed to write sample");

        sample_counter += 1;
    }

    writer.finalize().expect("Failed to finalize WAV writer");
}