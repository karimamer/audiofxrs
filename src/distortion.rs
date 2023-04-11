use std::env;
use std::f32::consts::PI;
use hound;

const SAMPLE_RATE: u32 = 44100;
const DURATION_SECS: u32 = 5;
const FREQUENCY: f32 = 440.0;
const GAIN: f32 = 0.5;
const NUM_DELAY_LINES: usize = 4;
const WET_DRY_MIX: f32 = 0.5;
const DISTORTION_GAIN: f32 = 2.0;

fn soft_clip(x: f32) -> f32 {
    x.tanh()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_wav_file> <output_wav_file>", args[0]);
        return;
    }
    let input_file = &args[1];
    let output_file = &args[2];

    let mut reader = hound::WavReader::open(input_file).expect("Failed to open input WAV file");
    let spec = reader.spec();

    let mut writer = hound::WavWriter::create(output_file, spec).expect("Failed to create output WAV file");
    let mut delay_lines: Vec<Vec<f32>> = vec![vec![0.0; SAMPLE_RATE as usize]; NUM_DELAY_LINES];
    let mut write_heads: Vec<usize> = vec![0; NUM_DELAY_LINES];
    let mut read_heads: Vec<usize> = vec![0; NUM_DELAY_LINES];
    let mut sample_counter: u32 = 0;

    for (i, result) in reader.samples::<i16>().enumerate() {
        let s = result.expect("Failed to read sample");

        let t = i as f32 / SAMPLE_RATE as f32;
        let s_distorted = (s as f32 * DISTORTION_GAIN).tanh(); // Apply the distortion effect

        let num_channels = delay_lines.len();
        if sample_counter % (SAMPLE_RATE * (num_channels as u32)) == 0 {
            for (j, read_head) in read_heads.iter_mut().enumerate() {
                *read_head = (write_heads[j] + SAMPLE_RATE as usize - ((j + 1) * SAMPLE_RATE as usize / (num_channels + 1))) % (SAMPLE_RATE as usize);
            }
        }

        let mut out_sample = s_distorted;
        for (j, delay_line) in delay_lines.iter_mut().enumerate() {
            let delayed_sample = delay_line[read_heads[j]];
            out_sample += delayed_sample * WET_DRY_MIX;
            delay_line[write_heads[j]] = (delayed_sample + s_distorted) * WET_DRY_MIX;
            write_heads[j] = (write_heads[j] + 1) % delay_line.len();
            read_heads[j] = (read_heads[j] + 1) % delay_line.len();
        }

        let out_sample = (out_sample / (1.0 + WET_DRY_MIX * NUM_DELAY_LINES as f32)) as i16;
        writer.write_sample(out_sample).unwrap();
        sample_counter += 1;
    }
}
