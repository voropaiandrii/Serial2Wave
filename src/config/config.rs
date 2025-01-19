use std::fs;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub serial_port: String,
    pub serial_port_baud_rate: usize,
    pub sample_rate: usize,
    pub audio_frame_bytes_length: usize,
    pub audio_frame_number_bytes_length: usize,
    pub number_of_channels: usize,
    pub bytes_per_channel: usize,
    pub sync_bytes: Vec<u8>,
    pub output_files_prefix: String,
    pub output_wav_file_path: String, 
    pub output_log_file_path: String,
}

/// Struct to manage configuration file operations
pub struct ConfigManager {
    pub config_path: String,
}

impl ConfigManager {
    pub fn new(config_path: &str) -> Self {
        Self {
            config_path: config_path.to_string(),
        }
    }

    pub fn is_config_exist(&mut self) -> bool {
        Path::new(&self.config_path).exists()
    }

    pub fn create_default_config(&mut self) {
        let default_config = Config {
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

        let json_config = serde_json::to_string_pretty(&default_config)
        .expect("Failed to serialize default config to JSON");

        let mut file = File::create(&self.config_path)
            .expect("Failed to create configuration file");
        file.write_all(json_config.as_bytes())
            .expect("Failed to write default configuration to file");

        println!("✅ Default config file created at {}", self.config_path);
    }

    /// Load the configuration from the file
    pub fn load_config(&mut self) -> Config {
        let config_data = fs::read_to_string(&self.config_path)
            .expect("Failed to read configuration file");
        let config: Result<Config, serde_json::Error> = serde_json::from_str(&config_data);

        match config {
            Ok(config) => {
                self.ensure_folder_exists(&config.output_wav_file_path);
                self.ensure_folder_exists(&config.output_log_file_path);
                config // Return the successfully parsed Config
            }
            Err(e) => {
                panic!("❌ Failed to parse configuration file: {}", e);
            }
        }
    }

    fn ensure_folder_exists(&self, output_file_path: &str) {
        // Convert the string path to a PathBuf
        let path = Path::new(output_file_path);
    
        // Extract the parent directory (folder path)
        // if let Some(parent) = path.parent() {
        //     if !parent.exists() {
        //         println!("Folder '{}' does not exist. Creating it...", parent.display());
        //         fs::create_dir_all(parent).expect("Failed to create output folder");
        //     } else {
        //         println!("Folder '{}' already exists.", parent.display());
        //     }
        // } else {
        //     println!("⚠️ Warning: No parent folder found in the provided path '{}'", output_file_path);
        // }
    
        if !path.exists() {
            println!("Folder '{}' does not exist. Creating it...", path.display());
            fs::create_dir_all(path).expect("Failed to create output folder");
        } else {
            println!("Folder '{}' already exists.", path.display());
        }

    }
}