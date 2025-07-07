# Multi-Channel Audio Support

This document describes the multi-channel audio support implemented in the PineBuds Audio Receiver Project.

## Overview

The application now supports 1, 2, 3, and more audio channels according to the `config.number_of_channels` configuration parameter. The audio data is processed from **planar format** (separate channel blocks) and converted to **interleaved format** (samples mixed by channel) for WAV file output.

## Audio Data Format

### Input Format (Planar)
The incoming audio data is in **planar format**, where each channel's data is stored sequentially in separate blocks:

```
[Channel1_block][Channel2_block][Channel3_block]...
```

For example, with 3 channels and 3000 samples per channel:
- Bytes 0-5999: Channel 1 samples (3000 samples × 2 bytes)
- Bytes 6000-11999: Channel 2 samples (3000 samples × 2 bytes)  
- Bytes 12000-17999: Channel 3 samples (3000 samples × 2 bytes)

### Output Format (Interleaved)
The WAV file requires **interleaved format**, where samples from different channels are mixed together:

```
[Sample1_Ch1][Sample1_Ch2][Sample1_Ch3][Sample2_Ch1][Sample2_Ch2][Sample2_Ch3]...
```

## Configuration

### Key Parameters

- `number_of_channels`: Number of audio channels (1, 2, 3, 4, etc.)
- `bytes_per_channel`: Bytes per sample per channel (typically 2 for 16-bit audio)
- `audio_frame_bytes_length`: Total length of audio data in bytes per frame

### Frame Structure

The audio frame structure is:
```
[Audio Data] + [Frame Number] + [Sync Bytes]
```

Where:
- **Audio Data**: `audio_frame_bytes_length` bytes of planar audio samples
- **Frame Number**: 4 bytes (little-endian)
- **Sync Bytes**: 8 bytes synchronization pattern

## Implementation Details

### Planar to Interleaved Conversion

The audio data processing in `src/main.rs` converts planar format to interleaved format:

```rust
// Convert planar format to interleaved format
let mut interleaved_samples: Vec<i16> = Vec::with_capacity(total_samples);

// Process each sample position across all channels
for sample_idx in 0..samples_per_channel {
    for channel in 0..config.number_of_channels {
        // Calculate the byte offset for this channel and sample
        let channel_offset = channel * samples_per_channel * bytes_per_sample;
        let sample_offset = sample_idx * bytes_per_sample;
        let byte_offset = channel_offset + sample_offset;
        
        // Extract the sample bytes for this channel and sample
        let sample_bytes = &data[byte_offset..byte_offset + bytes_per_sample];
        
        // Convert bytes to sample value
        let sample_value = match bytes_per_sample {
            2 => LittleEndian::read_i16(sample_bytes),
            4 => LittleEndian::read_i32(sample_bytes) as i16,
            _ => panic!("Unsupported bytes per channel: {}", bytes_per_sample)
        };
        
        interleaved_samples.push(sample_value);
    }
}
```

### WAV File Output

The WAV file is automatically configured with the correct number of channels:
```rust
let spec = hound::WavSpec {
    channels: config.number_of_channels.try_into().expect("config.number_of_channels is too large for u16"),
    sample_rate: config.sample_rate.try_into().expect("config.sample_rate is too large for u16"),
    bits_per_sample: (config.bytes_per_channel as u16) * 8u16,
    sample_format: hound::SampleFormat::Int,
};
```

The `hound` library automatically handles interleaved multi-channel data when writing samples.

## Example Configurations

### Three Channels (Your Current Configuration)
```json
{
    "number_of_channels": 3,
    "bytes_per_channel": 2,
    "audio_frame_bytes_length": 18000
}
```

**Data Layout:**
- Channel 1: 6000 bytes (3000 samples)
- Channel 2: 6000 bytes (3000 samples)  
- Channel 3: 6000 bytes (3000 samples)
- Total: 18000 bytes

### Single Channel (Mono)
```json
{
    "number_of_channels": 1,
    "bytes_per_channel": 2,
    "audio_frame_bytes_length": 4000
}
```

### Two Channels (Stereo)
```json
{
    "number_of_channels": 2,
    "bytes_per_channel": 2,
    "audio_frame_bytes_length": 8000
}
```

## Testing

The implementation includes comprehensive tests:

1. **Multi-channel configuration validation**: `tests/multi_channel_test.rs`
2. **Planar format conversion**: `tests/planar_format_test.rs`
3. **Original parser tests**: Existing tests for 1-channel and 3-channel data

Run tests with:
```bash
cargo test
```

## Usage

1. Configure the `number_of_channels` parameter in your config file
2. Ensure `audio_frame_bytes_length` is divisible by `bytes_per_channel * number_of_channels`
3. Run the application as usual

The application will automatically:
- Process planar format audio data
- Convert to interleaved format for WAV output
- Validate frame lengths
- Write properly formatted multi-channel WAV files
- Display channel information in logs

## Limitations

- Currently supports 2 and 4 bytes per channel (16-bit and 32-bit audio)
- Frame length must be divisible by `bytes_per_channel * number_of_channels`
- Audio data must be in planar format (separate channel blocks)
- Maximum 65535 channels (u16 limit for WAV format) 