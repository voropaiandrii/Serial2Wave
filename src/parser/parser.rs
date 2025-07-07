use std::io::{self, Read};
use std::collections::VecDeque;
use std::ptr::null;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::config::config;

#[derive(PartialEq, Debug)]
pub enum FrameType {
    LogData,
    AudioData,
}

pub struct Parser {
    config: config::Config,
    last_frame_number: u64,
    data_queue: VecDeque<u8>,
    callback: Option<Box<dyn Fn(FrameType, &[u8]) + Send + Sync>>,
}

impl Parser {
    // Constructor-like function to create a new ParserStruct
    pub fn new(config: config::Config) -> Self {
        Self {
            config,
            last_frame_number: 0,
            data_queue: VecDeque::new(),
            callback: None
        }
    }

    /// Set a callback function
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(FrameType, &[u8]) + Send + Sync + 'static,
    {
        self.callback = Some(Box::new(callback));
    }

    /// Push data into the parser's queue
    pub fn push_data(&mut self, data: &[u8]) {
        self.data_queue.extend(data);
    }

    // Process data in the queue
    pub fn process(&mut self) {
        if self.data_queue.len() >= self.config.audio_frame_bytes_length {
            let packet_to_review: Vec<u8> = self.data_queue.iter().take(self.data_queue.len()).copied().collect();
            let found_positions: Vec<usize> = packet_to_review.windows(self.config.sync_bytes.len())
                .enumerate() // Get index along with each window
                .filter_map(|(i, window)| {
                    if window == self.config.sync_bytes {
                        Some(i) // Return index if window matches needle
                    } else {
                        None
                    }
                })
                .collect();
            //print!("found_positions length: {}", found_positions.len());

            let mut last_audio_frame_position: usize = 0;

            for position in found_positions.iter() {
                let position = *position; // Explicitly dereference
                if position < (self.config.audio_frame_bytes_length + self.config.audio_frame_number_bytes_length) {
                    continue;
                }
                let log_end_index: usize = position - (self.config.audio_frame_bytes_length + self.config.audio_frame_number_bytes_length);
                let packet_logs_size = log_end_index - last_audio_frame_position;
                if packet_logs_size > 0 {
                    let packet_logs = &packet_to_review[last_audio_frame_position..log_end_index];
                    if let Some(callback) = &self.callback {
                        callback(FrameType::LogData, packet_logs);
                    }
                }
                let audio_packet = &packet_to_review[log_end_index..position + 8];
                    if let Some(callback) = &self.callback {
                        callback(FrameType::AudioData, audio_packet);
                    }
                last_audio_frame_position = position + self.config.sync_bytes.len();
            }

            // Remove the all found bytes from the queue
            if !found_positions.is_empty() {
                self.data_queue.drain(..*found_positions.last().unwrap() + self.config.sync_bytes.len());
            }

            // 1) Check queue stored length
            // 2) Is queue is bigger than 4012 start the proccessing
            // 3) Search for the audio frame sync bytes starting from the front bytes
            // 4) Check if the queue has 4004 bytes in front of the sync frame
            // 5) Copy all front bytes out of the audio frame bytes and mark them as LOG and delete them from the queue
            // 6) Copy the audio frame bytes and mark them as AUDIO and remove from the queue
        }
    }

    /// Start processing in a new thread
    pub fn start(parser: Arc<Mutex<Self>>) {
        thread::spawn(move || {
            loop {
                let mut parser = parser.lock().expect("Failed to lock parser mutex");
                parser.process();
                drop(parser); // Explicitly release the lock
                
                // Prevent busy-waiting
                thread::sleep(Duration::from_millis(10));
            }
        });
    }

    fn process_log(data: Vec<u8>) {

    }
    
    fn process_audio_frame(data: Vec<u8>) {
        
    }

    // pub fn extract_frame_number(packet: &[u8]) -> u64 {
    //     // TODO: fix the framing error let frame_number_bytes = &packet[3999..4007];
    //     //config.audio_frame_bytes_length - 1..config.audio_frame_bytes_length
    //     let frame_number_bytes = &packet[3999..4007];
    //     u64::from_le_bytes(frame_number_bytes.try_into().expect("Invalid frame number length"))
    // }
}

#[cfg(test)]
mod tests {
    use super::*; // Access parent module (`parser`) and its items
    use crate::utils::test_utils;
    use crate::config::config;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_parser() {
        // Path to the test data file
        let path = "tests/data/CoolTerm Capture (Untitled_1) 2024-12-21 12-24-34-830.txt";

        // Attempt to read the file
        let result = test_utils::read_file_as_bytes(path);

        // Assert that the file was read successfully
        assert!(result.is_ok(), "Failed to read the file");

        let data = result.unwrap();
        
        // Assert that the file is not empty
        assert!(!data.is_empty(), "File should not be empty");

        // Shared storage for callback results using Arc<Mutex>
        let callback_results = Arc::new(Mutex::new(Vec::<(FrameType, Vec<u8>)>::new()));
        let callback_results_clone = Arc::clone(&callback_results);

        let default_config = config::Config {
            serial_port: String::from("/dev/tty.usbmodem01234567891"),
            serial_port_baud_rate: 2_000_000,
            sample_rate: 48000,
            audio_frame_bytes_length: 4000,
            audio_frame_number_bytes_length: 4,
            number_of_channels: 1,
            bytes_per_channel: 2,
            sync_bytes: vec![0xFF, 0x01, 0xFF, 0x02, 0xFF, 0x03, 0xFF, 0x04],
            output_files_prefix: String::from("prefix"),
            output_wav_file_path: String::from("./"),
            output_log_file_path: String::from("./"),
        };

        let mut parser = Parser::new(default_config.clone());
        parser.set_callback(move |frame_type, data| {
            let mut results: std::sync::MutexGuard<'_, Vec<(FrameType, Vec<u8>)>> = callback_results_clone.lock().unwrap();
            results.push((frame_type, data.to_vec()));
        });

        // Start processing frames

        // 1️⃣ Logs + 1 broken audio frame + 1 correct audio frame + a big of next audio frame
        parser.push_data(&data[0..12000]); 
        parser.process();
        // 619 bytes left in the data_queue

        {
            let results = callback_results.lock().unwrap();
            assert!(
                results.len() == 3,
                "Expected at least 3 frames. Found: {}",
                results.len()
            );

            // Log
            // Audio - 7361
            // Audio - 11373
            assert_eq!(results[0].0, FrameType::LogData, "0 frame should be LogData");
            assert_eq!(results[1].0, FrameType::AudioData, "1 frame should be AudioData");
            //let result_array_1: &[u8] = &results[1].1;
            assert_eq!(results[1].1[4004..4012], default_config.sync_bytes, "1 frame should have sync_vec");

            let frame_number_1: u32 = (results[1].1[4000] as u32)
                    | ((results[1].1[4001] as u32) << 8)
                    | ((results[1].1[4002] as u32) << 16)
                    | ((results[1].1[4003] as u32) << 24);
            assert_eq!(0, frame_number_1, "1 frame should have frame number 0");

            assert_eq!(results[2].0, FrameType::AudioData, "2 frame should be AudioData");
            let frame_number_2: u32 = (results[2].1[4000] as u32)
            | ((results[2].1[4001] as u32) << 8)
            | ((results[2].1[4002] as u32) << 16)
            | ((results[2].1[4003] as u32) << 24);
            assert_eq!(1, frame_number_2, "2 frame should have frame number 1");
        }

        // 2️⃣ +2 correct audio frames
        parser.push_data(&data[12000..19500]);
        parser.process();
        // 95 bytes left in the data_queue

        {
            let results = callback_results.lock().unwrap();
            assert!(
                results.len() == 5,
                "Expected at least 5 frames. Found: {}",
                results.len()
            );

            // Log
            // Audio - 7361
            // Audio - 11373
            // Audio - 15385
            // Audio - 19397
            assert_eq!(results[0].0, FrameType::LogData, "0 frame should be LogData");
            assert_eq!(results[1].0, FrameType::AudioData, "1 frame should be AudioData");

            assert_eq!(results[2].0, FrameType::AudioData, "2 frame should be AudioData");
            assert_eq!(results[3].0, FrameType::AudioData, "3 frame should be AudioData");
            assert_eq!(results[3].1[4004..4012], default_config.sync_bytes, "3 frame should have sync_vec");
            let frame_number_3: u32 = (results[3].1[4000] as u32)
            | ((results[3].1[4001] as u32) << 8)
            | ((results[3].1[4002] as u32) << 16)
            | ((results[3].1[4003] as u32) << 24);
            assert_eq!(2, frame_number_3, "3 frame should have frame number 2");
            assert_eq!(results[4].0, FrameType::AudioData, "4 frame should be AudioData");
            assert_eq!(results[4].1[4004..4012], default_config.sync_bytes, "4 frame should have sync_vec");
            let frame_number_4: u32 = (results[4].1[4000] as u32)
            | ((results[4].1[4001] as u32) << 8)
            | ((results[4].1[4002] as u32) << 16)
            | ((results[4].1[4003] as u32) << 24);
            assert_eq!(3, frame_number_4, "4 frame should have frame number 3");
        }

        // 3️⃣ First part of a correct audio frame
        parser.push_data(&data[19500..21000]);
        parser.process();
        // 1595

        {
            let results = callback_results.lock().unwrap();
            assert!(
                results.len() == 5,
                "Expected still 5 frames after partial data. Found: {}",
                results.len()
            );
            // Log
            // Audio - 7361
            // Audio - 11373
            // Audio - 15385
            // Audio - 19397
            // Log
            // Audio - 23409
            assert_eq!(results[0].0, FrameType::LogData, "0 frame should be LogData");
            assert_eq!(results[1].0, FrameType::AudioData, "1 frame should be AudioData");
            assert_eq!(results[2].0, FrameType::AudioData, "2 frame should be AudioData");
            assert_eq!(results[3].0, FrameType::AudioData, "3 frame should be AudioData");
            assert_eq!(results[4].0, FrameType::AudioData, "4 frame should be AudioData");
        }

        // 4️⃣ Second part of a correct audio frame
        parser.push_data(&data[21000..24000]);
        parser.process();
        // 583 bytes left in the data_queue

        {
            let results = callback_results.lock().unwrap();
            assert!(
                results.len() == 6,
                "Expected 6 frames after completing partial data. Found: {}",
                results.len()
            );
            // Log
            // Audio - 7361
            // Audio - 11373
            // Audio - 15385
            // Audio - 19397
            // Log
            // Audio - 23409
            // Audio - 27421
            assert_eq!(results[0].0, FrameType::LogData, "0 frame should be LogData");
            assert_eq!(results[1].0, FrameType::AudioData, "1 frame should be AudioData");
            assert_eq!(results[2].0, FrameType::AudioData, "2 frame should be AudioData");
            assert_eq!(results[3].0, FrameType::AudioData, "3 frame should be AudioData");
            assert_eq!(results[4].0, FrameType::AudioData, "4 frame should be AudioData");
            assert_eq!(results[5].0, FrameType::AudioData, "6 frame should be AudioData");
            assert_eq!(results[5].1[4004..4012], default_config.sync_bytes, "6 frame should have sync_vec");
            let frame_number_5: u32 = (results[5].1[4000] as u32)
            | ((results[5].1[4001] as u32) << 8)
            | ((results[5].1[4002] as u32) << 16)
            | ((results[5].1[4003] as u32) << 24);
            assert_eq!(4, frame_number_5, "4 frame should have frame number 4");
        }

        // 5️⃣ +2 correct audio frames + logs
        parser.push_data(&data[24000..32000]);
        parser.process();

        {
            let results = callback_results.lock().unwrap();
            assert!(
                results.len() == 9,
                "Expected at least 9 frames (1 broken audio, 7 correct audio, logs). Found: {}",
                results.len()
            );
            // Log
            // Audio - 7361
            // Audio - 11373
            // Audio - 15385
            // Audio - 19397
            // Log
            // Audio - 23409
            // Audio - 27421
            // Log
            // Audio - 31505
            // Audio - 35517
            assert_eq!(results[0].0, FrameType::LogData, "0 frame should be LogData");
            assert_eq!(results[1].0, FrameType::AudioData, "1 frame should be AudioData");
            assert_eq!(results[2].0, FrameType::AudioData, "2 frame should be AudioData");
            assert_eq!(results[3].0, FrameType::AudioData, "3 frame should be AudioData");
            assert_eq!(results[4].0, FrameType::AudioData, "4 frame should be AudioData");
            assert_eq!(results[5].0, FrameType::AudioData, "5 frame should be AudioData");
            assert_eq!(results[6].0, FrameType::AudioData, "6 frame should be AudioData");
            assert_eq!(results[6].1[4004..4012], default_config.sync_bytes, "6 frame should have sync_vec");
            let frame_number_6: u32 = (results[6].1[4000] as u32)
            | ((results[6].1[4001] as u32) << 8)
            | ((results[6].1[4002] as u32) << 16)
            | ((results[6].1[4003] as u32) << 24);
            assert_eq!(5, frame_number_6, "6 frame should have frame number 5");
            assert_eq!(results[7].0, FrameType::LogData, "7 frame should be LogData");
            assert_eq!(results[8].0, FrameType::AudioData, "8 frame should be AudioData");
            assert_eq!(results[8].1[4004..4012], default_config.sync_bytes, "8 frame should have sync_vec");
            let frame_number_8: u32 = (results[8].1[4000] as u32)
            | ((results[8].1[4001] as u32) << 8)
            | ((results[8].1[4002] as u32) << 16)
            | ((results[8].1[4003] as u32) << 24);
            assert_eq!(6, frame_number_8, "8 frame should have frame number 6");

        }
    }

     #[test]
    fn test_parser_3ch() {
        // Path to the test data file
        let path = "tests/data/CoolTerm Capture (Untitled_0) 2025-07-07 23-03-15-968 3ch.txt";

        // Attempt to read the file
        let result = test_utils::read_file_as_bytes(path);

        // Assert that the file was read successfully
        assert!(result.is_ok(), "Failed to read the file");

        let data = result.unwrap();
        
        // Assert that the file is not empty
        assert!(!data.is_empty(), "File should not be empty");

        // Shared storage for callback results using Arc<Mutex>
        let callback_results = Arc::new(Mutex::new(Vec::<(FrameType, Vec<u8>)>::new()));
        let callback_results_clone = Arc::clone(&callback_results);

        let default_config = config::Config {
            serial_port: String::from("/dev/tty.usbmodem01234567891"),
            serial_port_baud_rate: 2_600_000,
            sample_rate: 48000,
            audio_frame_bytes_length: 18000,
            audio_frame_number_bytes_length: 4,
            number_of_channels: 3,
            bytes_per_channel: 2,
            sync_bytes: vec![0xFF, 0x01, 0xFF, 0x02, 0xFF, 0x03, 0xFF, 0x04],
            output_files_prefix: String::from("prefix"),
            output_wav_file_path: String::from("./"),
            output_log_file_path: String::from("./"),
        };

        let mut parser = Parser::new(default_config.clone());
        parser.set_callback(move |frame_type, data| {
            let mut results: std::sync::MutexGuard<'_, Vec<(FrameType, Vec<u8>)>> = callback_results_clone.lock().unwrap();
            results.push((frame_type, data.to_vec()));
        });

        // Start processing frames

        // 1️⃣ Logs + 1 audio frame
        parser.push_data(&data[17872..39156]); 
        parser.process();
        // 619 bytes left in the data_queue

        {
            let results = callback_results.lock().unwrap();
            assert!(
                results.len() == 2,
                "Expected at least 2 frames. Found: {}",
                results.len()
            );

            // Log
            // Audio - 7361
            // Audio - 11373
            assert_eq!(results[0].0, FrameType::LogData, "0 frame should be LogData");
            assert_eq!(results[1].0, FrameType::AudioData, "1 frame should be AudioData");
            //let result_array_1: &[u8] = &results[1].1;
            assert_eq!(results[1].1[18004..18012], default_config.sync_bytes, "1 frame should have sync_vec");

            let frame_number_1: u32 = (results[1].1[18000] as u32)
                    | ((results[1].1[18001] as u32) << 8)
                    | ((results[1].1[18002] as u32) << 16)
                    | ((results[1].1[18003] as u32) << 24);
            assert_eq!(0, frame_number_1, "1 frame should have frame number 0");
        }
    }
}
