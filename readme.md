# audiofxrs 
    
The aim of this project is just explore audio effects and learn rust by building an audio effects CLI tool. The project is is a work in progress and extremly far away from being something meaningful.

# Implemented:

**Distortion**: Distortion effects simulate the sound of an overdriven amplifier. This can be done using various methods such as clipping, waveshaping, or applying transfer functions to the input signal.

**Chorus**: Chorus adds richness to the sound by simulating multiple slightly detuned versions of the input signal. This can be achieved by using multiple delay lines with modulated delay times.

**Flanger**: A flanger creates a sweeping comb filter effect by mixing the input signal with a modulated delayed version of itself. It is similar to chorus but with a shorter delay time and higher feedback.

**Phaser**: A phaser creates a sweeping notch filter effect by modulating the phase of the input signal. This is often achieved by using an all-pass filter with a modulated delay time.

**Tremolo**: Tremolo is the modulation of the amplitude (volume) of the input signal at a specific frequency. This can be achieved by multiplying the input signal with a low-frequency oscillator (LFO) waveform, such as a sine wave.

**Vibrato**: Vibrato is the modulation of the pitch of the input signal at a specific frequency. This can be achieved by modulating the delay time of a delay line with a low-frequency oscillator (LFO).

**Equalization** (EQ): EQ is the process of adjusting the balance between different frequency components of the input signal. This can be achieved using various types of filters, such as low-pass, high-pass, band-pass, or notch filters.

**Compression**: Compression reduces the dynamic range of the input signal by attenuating the amplitude of loud signals and amplifying quiet signals. This can be done using various methods, such as peak, RMS, or multi-band compression.

**Pitch shifting**: Pitch shifting changes the pitch of the input signal without affecting its duration. This can be achieved using various algorithms, such as granular synthesis or phase vocoding.

Time stretching: Time stretching changes the duration of the input signal without affecting its pitch. This can be achieved using various algorithms, such as granular synthesis, phase vocoding, or the synchronized overlap-add (SOLA) method.

# To be implmented 
**Limiting**: Restricts the maximum amplitude of an audio signal to a specific threshold.

**Expander/Gate**: Increases the dynamic range of an audio signal by attenuating the volume of quiet parts and/or amplifying loud parts.

**Frequency Shifting**: Shifts all frequency components of an audio signal by a fixed amount.

**Ring Modulation**: Multiplies the input signal with another signal (usually a sine wave), creating inharmonic sidebands.

**Bitcrushing**: Reduces the bit depth and/or sample rate of an audio signal, introducing distortion and aliasing.

**Auto-Wah/Envelope Filter**: Modulates the frequency of a band-pass or low-pass filter based on the amplitude of the input signal.

**Stereo Widening**: Enhances the stereo image of an audio signal by manipulating the differences between the left and right channels.

**Convolution**: Processes the input signal with an impulse response, which can be used to apply reverberation, EQ, or other effects based on real or virtual spaces.



