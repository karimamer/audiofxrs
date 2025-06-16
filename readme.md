# AudioFXRS - Rust Audio Effects Processor

The aim of this project is just explore audio effects and learn rust by building an audio effects CLI tool.
A modular, command-line audio effects processor written in Rust. This project provides a unified interface for applying various audio effects to WAV files with configurable parameters.

## Features

### Implemented Effects

- **Chorus**: Adds richness by simulating multiple detuned versions of the input signal
- **Delay/Echo**: Creates echoes by delaying the input signal with configurable feedback and damping
- **Distortion**: Multiple distortion types including soft clip, hard clip, overdrive, and fuzz
- **Gate/Noise Gate**: Removes background noise by cutting off signals below a threshold
- **Limiter**: Prevents audio from exceeding a threshold with configurable attack/release times
- **Reverb**: Algorithmic reverb with configurable room size, damping, and pre-delay
- **Compression**: Dynamic range compression with configurable threshold, ratio, attack, and release
- **EQ**: 3-band equalizer with adjustable frequency bands and gains
- **Flanger**: Sweeping comb filter effect with modulated delay
- **Phaser**: Phase modulation effect creating moving notches in the frequency spectrum
- **Tremolo**: Amplitude modulation with multiple waveform shapes
- **Vibrato**: Pitch modulation effect
- **Pitch Shifting**: Change pitch without affecting duration (basic implementation)
- **Time Stretching**: Change duration without affecting pitch (basic implementation)

### Architecture

The codebase has been completely refactored with a modular design:

- **Unified CLI**: Single binary with effect selection via command line
- **Common Audio I/O**: Shared WAV file reading/writing with proper error handling
- **Effect Trait System**: All effects implement a common `AudioEffect` trait
- **Parameter System**: Type-safe parameter handling with validation
- **DSP Utilities**: Shared digital signal processing functions and delay lines

## Installation

```bash
git clone <repository-url>
cd audiofxrs
cargo build --release
```

## Usage

### Basic Syntax

```bash
audiofxrs <effect> <input.wav> <output.wav> [--param value]
```

### List Available Effects

```bash
audiofxrs --list
```

### Get Effect Information

```bash
audiofxrs --info <effect_name>
```

### Examples

```bash
# Apply chorus with custom parameters
audiofxrs chorus input.wav output.wav --rate 2.0 --depth 3.0 --mix 0.7

# Apply delay/echo
audiofxrs delay input.wav output.wav --delay 500 --feedback 0.4 --mix 0.5

# Apply gate/noise gate
audiofxrs gate input.wav output.wav --threshold 0.1 --release 100 --ratio 1.0

# Apply limiter
audiofxrs limiter input.wav output.wav --threshold 0.8 --attack 1.0 --release 50

# Apply distortion
audiofxrs distortion input.wav output.wav --gain 3.0 --type 1 --mix 0.8

# Apply reverb
audiofxrs reverb input.wav output.wav --room_size 0.8 --mix 0.4 --damping 0.6

# Apply compression
audiofxrs compression input.wav output.wav --threshold 0.3 --ratio 4.0 --attack 5.0

# Apply tremolo with triangle wave
audiofxrs tremolo input.wav output.wav --rate 8.0 --depth 0.6 --wave 1

# Get help for a specific effect
audiofxrs --info chorus
```

## Effect Parameters

### Chorus
- `--rate`: LFO rate in Hz (0.1 to 10.0, default: 0.5)
- `--depth`: Modulation depth in milliseconds (0.1 to 10.0, default: 2.0)
- `--mix`: Wet/dry mix (0.0 to 1.0, default: 0.5)
- `--feedback`: Feedback amount (0.0 to 0.9, default: 0.0)

### Delay
- `--delay`: Delay time in milliseconds (10.0 to 2000.0, default: 250.0)
- `--feedback`: Feedback amount (0.0 to 0.9, default: 0.3)
- `--mix`: Wet/dry mix (0.0 to 1.0, default: 0.3)
- `--damping`: High frequency damping of feedback (0.0 to 1.0, default: 0.2)

### Gate
- `--threshold`: Gate threshold (0.001 to 1.0, default: 0.1)
- `--attack`: Attack time in milliseconds (0.1 to 100.0, default: 1.0)
- `--hold`: Hold time in milliseconds (0.0 to 1000.0, default: 10.0)
- `--release`: Release time in milliseconds (1.0 to 5000.0, default: 100.0)
- `--ratio`: Gate ratio - 1.0 = full gate, 0.0 = no gate (0.0 to 1.0, default: 1.0)

### Limiter
- `--threshold`: Limiting threshold (0.1 to 1.0, default: 0.8)
- `--attack`: Attack time in milliseconds (0.1 to 10.0, default: 1.0)
- `--release`: Release time in milliseconds (1.0 to 500.0, default: 50.0)
- `--output`: Output gain (0.1 to 2.0, default: 1.0)

### Distortion
- `--gain`: Input gain amount (0.1 to 10.0, default: 2.0)
- `--threshold`: Distortion threshold (0.1 to 1.0, default: 0.7)
- `--mix`: Wet/dry mix (0.0 to 1.0, default: 1.0)
- `--output`: Output level (0.1 to 1.0, default: 0.8)
- `--type`: Distortion type (0=Soft, 1=Hard, 2=Overdrive, 3=Fuzz, default: 0)

### Reverb
- `--room_size`: Room size (0.1 to 1.0, default: 0.5)
- `--damping`: High frequency damping (0.0 to 1.0, default: 0.5)
- `--mix`: Wet/dry mix (0.0 to 1.0, default: 0.3)
- `--feedback`: Feedback amount (0.0 to 0.9, default: 0.5)
- `--pre_delay`: Pre-delay time in milliseconds (0.0 to 100.0, default: 20.0)

### Compression
- `--threshold`: Compression threshold (0.0 to 1.0, default: 0.5)
- `--ratio`: Compression ratio (1.0 to 20.0, default: 4.0)
- `--attack`: Attack time in milliseconds (0.1 to 100.0, default: 10.0)
- `--release`: Release time in milliseconds (10.0 to 1000.0, default: 100.0)
- `--makeup`: Makeup gain (0.1 to 4.0, default: 1.0)

### EQ
- `--low_gain`: Low frequency gain in dB (-12.0 to 12.0, default: 0.0)
- `--mid_gain`: Mid frequency gain in dB (-12.0 to 12.0, default: 0.0)
- `--high_gain`: High frequency gain in dB (-12.0 to 12.0, default: 0.0)
- `--low_freq`: Low/mid crossover frequency (100.0 to 1000.0, default: 300.0)
- `--high_freq`: Mid/high crossover frequency (1000.0 to 8000.0, default: 3000.0)

### Tremolo
- `--rate`: Tremolo rate in Hz (0.1 to 20.0, default: 5.0)
- `--depth`: Modulation depth (0.0 to 1.0, default: 0.7)
- `--wave`: Wave shape (0=Sine, 1=Triangle, 2=Square, 3=Sawtooth, default: 0)

## File Format Support

Currently supports:
- **Input**: 16-bit PCM WAV files
- **Output**: 16-bit PCM WAV files
- **Sample Rates**: 8 kHz to 192 kHz
- **Channels**: Mono and stereo (effect-dependent)

## Development

### Project Structure

```
src/
├── main.rs              # Entry point
├── cli.rs               # Command-line interface
├── audio_io.rs          # Audio file I/O utilities
└── effects/
    ├── mod.rs           # Effect trait and common utilities
    ├── chorus.rs        # Chorus effect implementation
    ├── delay.rs         # Delay/Echo effect implementation
    ├── distortion.rs    # Distortion effect implementation
    ├── gate.rs          # Gate/Noise Gate effect implementation
    ├── limiter.rs       # Limiter effect implementation
    ├── reverb.rs        # Reverb effect implementation
    ├── compression.rs   # Compression effect implementation
    ├── eq.rs           # EQ effect implementation
    ├── flanger.rs      # Flanger effect implementation
    ├── phaser.rs       # Phaser effect implementation
    ├── tremolo.rs      # Tremolo effect implementation
    ├── vibrato.rs      # Vibrato effect implementation
    ├── pitch_shifting.rs # Pitch shifting effect (basic)
    └── time_stretching.rs # Time stretching effect (basic)
```

### Adding New Effects

1. Create a new module in `src/effects/`
2. Implement the `AudioEffect` trait
3. Register the effect in `src/cli.rs`
4. Add tests following the existing patterns

### Running Tests

```bash
cargo test
```

### Building for Release

```bash
cargo build --release
```

## Future Improvements

- **Additional Effects**: Expansion, frequency shifting, ring modulation, bitcrushing
- **Better Algorithms**: Improved pitch shifting and time stretching using PSOLA or phase vocoder
- **Multi-channel Support**: Full surround sound processing
- **Real-time Processing**: Live audio processing capabilities
- **Plugin Format**: VST/AU plugin versions
- **GUI Interface**: Graphical user interface for parameter control

## Dependencies

- `hound`: WAV file I/O
- `dasp`: Digital audio signal processing utilities
- `biquad`: Digital filter implementations

## License

GNU General Public License v3.0 - see LICENSE file for details.
