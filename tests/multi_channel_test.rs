use std::sync::{Arc, Mutex};
use PineBudsAudioReceiverProject::config::config;
use PineBudsAudioReceiverProject::parser::parser::{Parser, FrameType};

#[test]
fn test_multi_channel_audio_processing() {
    // Test configurations for different channel counts
    let test_configs = vec![
        (1, 4000, "1 channel"),
        (2, 8000, "2 channels"), 
        (3, 12000, "3 channels"),
        (4, 16000, "4 channels"),
    ];

    for (channels, frame_length, description) in test_configs {
        println!("Testing {} with frame length {}", description, frame_length);
        
        let config = config::Config {
            serial_port: String::from("/dev/tty.test"),
            serial_port_baud_rate: 2_000_000,
            sample_rate: 48000,
            audio_frame_bytes_length: frame_length,
            audio_frame_number_bytes_length: 4,
            number_of_channels: channels,
            bytes_per_channel: 2,
            sync_bytes: vec![0xFF, 0x01, 0xFF, 0x02, 0xFF, 0x03, 0xFF, 0x04],
            output_files_prefix: String::from("test"),
            output_wav_file_path: String::from("./test_output"),
            output_log_file_path: String::from("./test_output"),
        };

        // Calculate expected values
        let bytes_per_sample = config.bytes_per_channel;
        let total_samples = config.audio_frame_bytes_length / bytes_per_sample;
        let samples_per_channel = total_samples / config.number_of_channels;

        println!("  - Total samples: {}", total_samples);
        println!("  - Samples per channel: {}", samples_per_channel);
        println!("  - Bytes per sample: {}", bytes_per_sample);

        // Validate frame length
        assert_eq!(
            config.audio_frame_bytes_length % (bytes_per_sample * config.number_of_channels),
            0,
            "Frame length {} should be divisible by bytes_per_sample * channels ({} * {} = {})",
            config.audio_frame_bytes_length,
            bytes_per_sample,
            config.number_of_channels,
            bytes_per_sample * config.number_of_channels
        );

        // Test that samples per channel calculation is correct
        assert_eq!(
            samples_per_channel * config.number_of_channels,
            total_samples,
            "Samples per channel * number of channels should equal total samples"
        );

        // Test that the frame length makes sense for the configuration
        assert!(
            samples_per_channel > 0,
            "Should have at least 1 sample per channel"
        );

        println!("  ✓ {} configuration is valid", description);
    }
}

#[test]
fn test_audio_data_processing_simulation() {
    let config = config::Config {
        serial_port: String::from("/dev/tty.test"),
        serial_port_baud_rate: 2_000_000,
        sample_rate: 48000,
        audio_frame_bytes_length: 6000, // 3 channels * 1000 samples * 2 bytes
        audio_frame_number_bytes_length: 4,
        number_of_channels: 3,
        bytes_per_channel: 2,
        sync_bytes: vec![0xFF, 0x01, 0xFF, 0x02, 0xFF, 0x03, 0xFF, 0x04],
        output_files_prefix: String::from("test"),
        output_wav_file_path: String::from("./test_output"),
        output_log_file_path: String::from("./test_output"),
    };

    // Simulate audio data processing
    let bytes_per_sample = config.bytes_per_channel;
    let total_samples = config.audio_frame_bytes_length / bytes_per_sample;
    let samples_per_channel = total_samples / config.number_of_channels;

    println!("3-channel audio processing simulation:");
    println!("  - Frame length: {} bytes", config.audio_frame_bytes_length);
    println!("  - Total samples: {}", total_samples);
    println!("  - Samples per channel: {}", samples_per_channel);
    println!("  - Channels: {}", config.number_of_channels);

    // Create simulated audio data (interleaved)
    let mut simulated_data = Vec::new();
    for sample_idx in 0..samples_per_channel {
        for channel in 0..config.number_of_channels {
            // Create a simple sine wave pattern for each channel
            let sample_value = (sample_idx as f32 * 0.1 + channel as f32 * 100.0) as i16;
            simulated_data.extend_from_slice(&sample_value.to_le_bytes());
        }
    }

    // Add frame number bytes
    let frame_number: u32 = 123;
    simulated_data.extend_from_slice(&frame_number.to_le_bytes());

    // Add sync bytes
    simulated_data.extend_from_slice(&config.sync_bytes);

    println!("  - Simulated data length: {} bytes", simulated_data.len());
    println!("  - Expected frame length: {} bytes", config.audio_frame_bytes_length + 4 + 8);

    // Verify the simulated data length
    assert_eq!(
        simulated_data.len(),
        config.audio_frame_bytes_length + config.audio_frame_number_bytes_length + config.sync_bytes.len(),
        "Simulated data length should match expected frame structure"
    );

    // Test processing the simulated data
    let callback_results: Arc<Mutex<Vec<(FrameType, Vec<u8>)>>> = Arc::new(Mutex::new(Vec::new()));
    let callback_results_clone = Arc::clone(&callback_results);

    let mut parser = Parser::new(config.clone());
    parser.set_callback(move |frame_type, data| {
        let mut results = callback_results_clone.lock().unwrap();
        results.push((frame_type, data.to_vec()));
    });

    // Process the simulated data
    parser.push_data(&simulated_data);
    parser.process();

    // Check results
    let results = callback_results.lock().unwrap();
    assert_eq!(results.len(), 1, "Should have processed one audio frame");
    assert_eq!(results[0].0, FrameType::AudioData, "Should be an audio frame");

    let audio_data = &results[0].1;
    assert_eq!(
        audio_data.len(),
        config.audio_frame_bytes_length + config.audio_frame_number_bytes_length + config.sync_bytes.len(),
        "Audio data length should match expected frame structure"
    );

    println!("  ✓ Audio data processing simulation successful");
} 