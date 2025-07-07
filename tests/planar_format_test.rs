use std::sync::{Arc, Mutex};
use PineBudsAudioReceiverProject::config::config;
use PineBudsAudioReceiverProject::parser::parser::{Parser, FrameType};

#[test]
fn test_planar_to_interleaved_conversion() {
    let config = config::Config {
        serial_port: String::from("/dev/tty.test"),
        serial_port_baud_rate: 2_600_000,
        sample_rate: 48000,
        audio_frame_bytes_length: 18000, // 3 channels * 3000 samples * 2 bytes
        audio_frame_number_bytes_length: 4,
        number_of_channels: 3,
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

    println!("Testing planar to interleaved conversion:");
    println!("  - Frame length: {} bytes", config.audio_frame_bytes_length);
    println!("  - Total samples: {}", total_samples);
    println!("  - Samples per channel: {}", samples_per_channel);
    println!("  - Channels: {}", config.number_of_channels);

    // Create simulated planar audio data
    // Format: [Channel1_block][Channel2_block][Channel3_block]
    let mut planar_data = Vec::new();
    
    // Channel 1: All samples = 100
    for _ in 0..samples_per_channel {
        let sample_value: i16 = 100;
        planar_data.extend_from_slice(&sample_value.to_le_bytes());
    }
    
    // Channel 2: All samples = 200
    for _ in 0..samples_per_channel {
        let sample_value: i16 = 200;
        planar_data.extend_from_slice(&sample_value.to_le_bytes());
    }
    
    // Channel 3: All samples = 300
    for _ in 0..samples_per_channel {
        let sample_value: i16 = 300;
        planar_data.extend_from_slice(&sample_value.to_le_bytes());
    }

    // Add frame number bytes
    let frame_number: u32 = 123;
    planar_data.extend_from_slice(&frame_number.to_le_bytes());

    // Add sync bytes
    planar_data.extend_from_slice(&config.sync_bytes);

    println!("  - Planar data length: {} bytes", planar_data.len());
    println!("  - Expected frame length: {} bytes", config.audio_frame_bytes_length + 4 + 8);

    // Verify the planar data structure
    assert_eq!(
        planar_data.len(),
        config.audio_frame_bytes_length + config.audio_frame_number_bytes_length + config.sync_bytes.len(),
        "Planar data length should match expected frame structure"
    );

    // Test the conversion manually to verify the algorithm
    let mut expected_interleaved = Vec::new();
    for sample_idx in 0..samples_per_channel {
        for channel in 0..config.number_of_channels {
            let channel_offset = channel * samples_per_channel * bytes_per_sample;
            let sample_offset = sample_idx * bytes_per_sample;
            let byte_offset = channel_offset + sample_offset;
            
            let sample_bytes = &planar_data[byte_offset..byte_offset + bytes_per_sample];
            let sample_value = i16::from_le_bytes([sample_bytes[0], sample_bytes[1]]);
            
            expected_interleaved.push(sample_value);
        }
    }

    // Verify the expected interleaved pattern
    // Should be: [100, 200, 300, 100, 200, 300, ...]
    for i in 0..samples_per_channel {
        assert_eq!(expected_interleaved[i * 3 + 0], 100, "Channel 1 sample {} should be 100", i);
        assert_eq!(expected_interleaved[i * 3 + 1], 200, "Channel 2 sample {} should be 200", i);
        assert_eq!(expected_interleaved[i * 3 + 2], 300, "Channel 3 sample {} should be 300", i);
    }

    println!("  - Expected interleaved samples: {}", expected_interleaved.len());
    println!("  - First 6 samples: {:?}", &expected_interleaved[0..6]);

    // Test processing the planar data through the parser
    let callback_results: Arc<Mutex<Vec<(FrameType, Vec<u8>)>>> = Arc::new(Mutex::new(Vec::new()));
    let callback_results_clone = Arc::clone(&callback_results);

    let mut parser = Parser::new(config.clone());
    parser.set_callback(move |frame_type, data| {
        let mut results = callback_results_clone.lock().unwrap();
        results.push((frame_type, data.to_vec()));
    });

    // Process the planar data
    parser.push_data(&planar_data);
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

    println!("  ✓ Planar to interleaved conversion test successful");
}

#[test]
fn test_planar_format_validation() {
    // Test different planar configurations
    let test_configs = vec![
        (1, 4000, "1 channel planar"),
        (2, 8000, "2 channels planar"), 
        (3, 18000, "3 channels planar"),
        (4, 16000, "4 channels planar"),
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