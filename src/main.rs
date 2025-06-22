use std::io::{self, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serialport::SerialPort;
use chrono::Local;
use argh::FromArgs;
use once_cell::sync::Lazy;
use hound;
use ctrlc;
use byteorder_slice::{ByteOrder, LittleEndian};
use std::process;

mod constants;
mod parser;
mod utils;
mod config;

/// An example CLI tool.
#[derive(FromArgs)]
struct Args {
    /// path to the configuration file
    #[argh(option, short = 'c', default = "String::from(\"./config.json\")")]
    config: String,

    /// verbose mode
    #[argh(switch, short = 'v')]
    verbose: bool,
}

fn current_millis() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis();
    millis.to_string()
}

fn clear_serial_buffer(port: &mut Box<dyn SerialPort>, bytes_to_clear: usize) {
    let mut discard_buffer = vec![0u8; bytes_to_clear];
    let mut total_bytes_read = 0;

    while total_bytes_read < bytes_to_clear {
        match port.read(&mut discard_buffer) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    break;
                }
                total_bytes_read += bytes_read;
            }
            Err(e) => {
                eprintln!("Error while clearing buffer: {}", e);
                break;
            }
        }
    }
}

// Global static variable for arguments
pub static ARGS: Lazy<Mutex<Args>> = Lazy::new(|| {
    let args: Args = argh::from_env();
    Mutex::new(args)
});

pub fn load_args() {
    let args = ARGS.lock().unwrap();
}

fn main() -> io::Result<()> {

    load_args();
    let args = ARGS.lock().unwrap();
    println!("Using config file: {}", args.config);
    if args.verbose {
        println!("Verbose mode is enabled.");
    }

    let mut config_manager = config::config::ConfigManager::new(&args.config);

    if !config_manager.is_config_exist() {
        println!("⚠️ Config file not found. Creating default...");
        config_manager.create_default_config();
    }
    println!("Loading the config file...");
    let config: config::config::Config = config_manager.load_config();

    // Serialize the config struct into a JSON string
    let json_config = serde_json::to_string_pretty(&config)
        .expect("Failed to serialize Config to JSON");
    println!("Loaded Config: {}", json_config);


    let now: chrono::DateTime<Local> = Local::now();
    let audio_file_name_str = format!("{}/{}_audio_{}.wav", 
        config.output_wav_file_path.clone(), 
        config.output_files_prefix.clone(),
        now.format("%Y-%m-%d-%H-%M-%S%.3f").to_string());
    let audio_file_name = Arc::new(audio_file_name_str);

    let ram_buffer = Arc::new(Mutex::new(Vec::new()));
    let ram_buffer_clone = Arc::clone(&ram_buffer);
    // Shared mutable writer initialized outside the block
    let writer = Arc::new(Mutex::new(None));

    let port = serialport::new(config.clone().serial_port.clone(), 
            config.clone().serial_port_baud_rate.try_into().expect("config.number_of_channels is too large for u32"))
        .timeout(Duration::from_secs(1))
        .open();

    let port_clone = port.as_ref();

    if port_clone.is_ok() {
        let spec = hound::WavSpec {
            channels: config.number_of_channels.try_into().expect("config.number_of_channels is too large for u16"),
            sample_rate: config.sample_rate.try_into().expect("config.sample_rate is too large for u16"),
            bits_per_sample: (config.bytes_per_channel as u16) * 8u16,
            sample_format: hound::SampleFormat::Int,
        };

        let wav_writer = hound::WavWriter::create(Path::new(&*audio_file_name), spec)
        .expect("Failed to create WAV writer");

        // Store the writer in the Arc<Mutex<Option<>>>
        *writer.lock().unwrap() = Some(wav_writer);

        println!("Serial port opened successfully and WAV writer initialized.");
    } else {
        eprintln!("Failed to open serial port: {}", port_clone.err().unwrap());
        process::exit(1);
    }

    let sync_vec: Vec<u8> = vec![0xFF, 0x01, 0xFF, 0x02, 0xFF, 0x03, 0xFF, 0x04];
    let parser = Arc::new(Mutex::new(parser::parser::Parser::new(config.clone())));

    // Set a callback to handle parsed frames
    {
        let ram_buffer_for_callback = Arc::clone(&ram_buffer); // Clone for the closure
        let mut parser_lock = parser.lock().unwrap();
        parser_lock.set_callback( move | frame_type, data| {
            match frame_type {
                parser::parser::FrameType::LogData => {
                    let now = Local::now();
                    let filtered_string: String = data.iter()
                        .filter(|&&b| b.is_ascii()) // Keep only ASCII bytes
                        .map(|&b| b as char)        // Convert each byte to a char
                        .collect(); 
                    println!("{} - {}", now.format("%Y-%m-%d %H:%M:%S%.3f"), filtered_string)
                },
                parser::parser::FrameType::AudioData => {
                    let mut buffer = ram_buffer_for_callback.lock().unwrap();
                    let now = Local::now();
                    let frame_number: u32 = (data[4000] as u32)
                    | ((data[4001] as u32) << 8)
                    | ((data[4002] as u32) << 16)
                    | ((data[4003] as u32) << 24);

                    // Convert byte chunks to i16 values
                    let data_i16: Vec<i16> = data[0..4000].chunks_exact(2) // Process chunks of two bytes
                    .map(|chunk| LittleEndian::read_i16(chunk))
                    .collect(); // Collect into Vec<i16>

                    // Extend the buffer with the converted i16 values
                    buffer.extend_from_slice(&data_i16);
                    println!("{} - AUDIO Frame Received Length: {}, frame_number: {}", now.format("%Y-%m-%d %H:%M:%S%.3f"), data.len(), frame_number); 
                },
            }
        });
    }

    // Start processing thread
    parser::parser::Parser::start(Arc::clone(&parser));

    // Clone the writer for Ctrl+C handler
    let writer_clone = Arc::clone(&writer);
    ctrlc::set_handler(move || {
        // Lock the buffer to access samples
        let buffer = ram_buffer_clone.lock().unwrap();

        if(buffer.len() == 0) {
            println!("\nNo audio data to flush! Exiting...");
            std::process::exit(0);
        }

        let mut writer_guard = writer_clone.lock().unwrap();

        println!("\nCtrl+C detected! Flushing buffer to {} file...", audio_file_name);
       
        if let Some(ref mut writer) = *writer_guard {
            for &sample in buffer.iter() {
                writer.write_sample(sample).expect("Failed to write sample");
            }

            writer.flush().expect("Failed to flush writer");
            println!("Audio data written successfully!");
        } else {
            eprintln!("Writer was not initialized.");
        }
        std::process::exit(0);
    }).expect("Error setting Ctrl+C handler");

    match port {
        Ok(mut port) => {
            println!("Listening on {} at {} baud...", config.serial_port.clone(), 
                config.serial_port_baud_rate);
            
            // Clear the serial buffer before starting
            clear_serial_buffer(&mut port, constants::common::SERIAL_READ_SIZE);

            let mut read_buffer: [u8; constants::common::SERIAL_READ_SIZE] = [0u8; constants::common::SERIAL_READ_SIZE]; // choose an appropriate size
            loop {
                match port.read(&mut read_buffer) {
                    Ok(n) if n > 0 => {
                        let mut parser = parser.lock().expect("Failed to lock parser mutex");
                        parser.push_data(&read_buffer[..n]);
                    }
                    Ok(_) => {
                        // n == 0 means EOF or no data; depending on serial config
                    }
                    Err(e) => {
                        //eprintln!("Serial read error: {}", e);
                        // Possibly break or handle error
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open serial port: {}", e);
        }
    }

    Ok(())
}
