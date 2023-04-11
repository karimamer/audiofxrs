/*Compression is a process that reduces the dynamic range of an audio signal by attenuating the amplitude of the signal above a certain threshold. 
threshold: The level above which compression occurs. A lower value will affect more of the signal, while a higher value will affect less. 
            It's set in the range of [0.0, 1.0], where 0.0 corresponds to the minimum amplitude and 1.0 to the maximum amplitude.
ratio: Determines the amount of compression applied to the signal above the threshold. 
        A higher ratio results in more aggressive compression, while a lower ratio results in gentler compression. 
        A ratio of 1:1 means no compression is applied, while a ratio of ∞:1 means that the output level will not increase beyond the threshold.

 */
use std::env;
use hound;
use dasp::Frame;

const SAMPLE_RATE: f64 = 44_100.0;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: compressor <input_wav> <output_wav>");
        return;
    }
    let input_file = &args[1];
    let output_file = &args[2];

    let mut reader = hound::WavReader::open(input_file).expect("Failed to open input WAV file");
    let spec = reader.spec();
    let mut writer = hound::WavWriter::create(output_file, spec).expect("Failed to create output WAV file");

    let threshold = 0.5; // Threshold for compression in the range [0.0, 1.0]
    let ratio = 4.0; // Compression ratio, higher values result in more aggressive compression

    for sample_result in reader.samples::<i16>() {
        let s = sample_result.expect("Failed to read sample");
        let s_f32 = s as f32 / 32_768.0;

        // Apply compression
        let compressed_sample = if s_f32.abs() > threshold {
            let gain_reduction = (s_f32.abs() - threshold) / ratio;
            s_f32.signum() * (threshold + gain_reduction)
        } else {
            s_f32
        };

        let out_sample = compressed_sample;
        let out_sample_i16 = (out_sample * 32_767.0).max(-32_768.0).min(32_767.0) as i16;
        writer.write_sample(out_sample_i16).expect("Failed to write sample");
    }

    writer.finalize().expect("Failed to finalize WAV writer");
}