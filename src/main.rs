// Import the required libraries and set constant values:
use hound;
use std::env;

const SAMPLE_RATE: u32 = 44100;
const DELAY_TIME_MS: f64 = 200.0;
const FEEDBACK: f32 = 0.5;
const WET_DRY_MIX: f32 = 0.5;
const NUM_DELAY_LINES: usize = 3;

fn main() {
    // Parse command line arguments for the output WAV file path:
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input.wav> <output.wav>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[2];

    // Read input WAV file
    let mut reader = hound::WavReader::open(input_file).unwrap();
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
    let num_channels = reader.spec().channels as usize;

    // Prepare output WAV file
    let spec = hound::WavSpec {
        channels: reader.spec().channels,
        sample_rate: reader.spec().sample_rate,
        bits_per_sample: reader.spec().bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(output_file, spec).unwrap();

     // Initialize multiple delay line buffers with variable lengths
     let mut delay_lengths = vec![
        (SAMPLE_RATE as f64 * (DELAY_TIME_MS / 1000.0)) as usize,
        (SAMPLE_RATE as f64 * (DELAY_TIME_MS / 1200.0)) as usize,
        (SAMPLE_RATE as f64 * (DELAY_TIME_MS / 1400.0)) as usize,
    ];
    let mut delay_lines: Vec<Vec<f32>> = delay_lengths
        .iter()
        .map(|&length| vec![0.0; length * num_channels])
        .collect();

    // Process samples and apply reverb
    let mut sample_counter = 0;
    for (i, sample) in samples.iter().enumerate() {
        let input_sample = *sample as f32;
        let channel = i % num_channels;

        // Update delay lengths periodically
        if sample_counter % (SAMPLE_RATE * (num_channels as u32)) == 0 {
            // You can use user input, an algorithm, or any other method to update delay_lengths
            // For demonstration purposes, we simply increase each delay length by 100 samples
            for (j, delay_length) in delay_lengths.iter_mut().enumerate() {
                *delay_length += 100;

                // Update delay_lines with the new delay lengths
                let new_length = *delay_length * num_channels;
                delay_lines[j].resize(new_length, 0.0);
            }
        }

        // Process each delay line
        let mut wet_sample = 0.0;
        for (delay_line, &_delay_length) in delay_lines.iter_mut().zip(delay_lengths.iter()) {
            let delayed_sample = delay_line[channel];

            // Combine input and delayed samples
            wet_sample += input_sample * (1.0 - WET_DRY_MIX) + delayed_sample * WET_DRY_MIX;

            // Update delay line with feedback
            let delay_input = input_sample + delayed_sample * FEEDBACK;
            delay_line[channel] = delay_input;

            // Shift delay line
            delay_line.rotate_right(num_channels);
        }

        wet_sample /= NUM_DELAY_LINES as f32; // Normalize the wet_sample
        let output_sample = wet_sample as i16;
        writer.write_sample(output_sample).unwrap();

        sample_counter += 1;
    }

    writer.finalize().unwrap();
    println!("Reverb effect applied. Check the output file: {}", output_file);
}