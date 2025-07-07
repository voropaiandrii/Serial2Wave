use std::fs::File;
use std::io::{Read, BufReader};
use std::path::Path;

use PineBudsAudioReceiverProject::utils::test_utils;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_file_as_bytes() {
        // Path to the test data file
        let path = "tests/data/CoolTerm Capture (Untitled_1) 2024-12-21 12-24-34-830.txt";

        // Attempt to read the file
        let result = test_utils::read_file_as_bytes(path);

        // Assert that the file was read successfully
        assert!(result.is_ok(), "Failed to read the file");

        let data = result.unwrap();
        
        // Assert that the file is not empty
        assert!(!data.is_empty(), "File should not be empty");

        // Print the contents for debugging (optional)
        println!("Read {} bytes from the file", data.len());
    }
}