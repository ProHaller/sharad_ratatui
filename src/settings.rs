use async_openai::error::OpenAIError;
// Import necessary libraries and modules for API interaction, file I/O, and serialization.
use async_openai::{config::OpenAIConfig, Client};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};

use crate::error::send_global_error;

// Define a structure to hold application settings with serialization and deserialization capabilities.
#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub language: String, // Preferred language setting for the application.
    pub openai_api_key: Option<String>, // Optional API key for OpenAI services.
    pub model: String,
    pub audio_output_enabled: bool, // Flag to enable or disable audio output.
    pub audio_input_enabled: bool,  // Flag to enable or disable audio input.
    pub debug_mode: bool,           // Flag to enable or disable debug mode.
}

// TODO:  Add a model parameter to change the AI model

// Implement the Default trait for Settings to provide a method to create default settings.
impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "English".to_string(), // Default language setting.
            openai_api_key: None,            // No API key by default.
            model: "gpt-4o-mini".to_string(),
            audio_output_enabled: true, // Audio output enabled by default.
            audio_input_enabled: true,  // Audio input enabled by default.
            debug_mode: false,          // Debug mode disabled by default.
        }
    }
}

// Additional implementation block for Settings.
impl Settings {
    // Constructor function to create new settings with default values.
    pub fn new() -> Self {
        Self::default()
    }

    // Load settings from a default file path.
    pub fn load() -> io::Result<Self> {
        Self::load_settings_from_file("./data/settings.json")
    }

    // Save current settings to a default file path.
    pub fn save(&self) -> io::Result<()> {
        std::fs::create_dir_all("./data")?; // Ensure the data directory exists.
        self.save_to_file("./data/settings.json")
    }

    // Load settings from a specified file path.
    pub fn load_settings_from_file(path: &str) -> io::Result<Self> {
        let data = fs::read_to_string(path)?; // Read settings from file.
        let settings = serde_json::from_str(&data)?; // Deserialize JSON data into settings.
        Ok(settings)
    }

    // Save current settings to a specified file path.
    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let data = serde_json::to_string_pretty(self)?; // Serialize settings into pretty JSON format.
        if let Some(parent) = std::path::Path::new(path).parent() {
            fs::create_dir_all(parent)?; // Create the directory if it doesn't exist.
        }
        let mut file = fs::File::create(path)?; // Create or overwrite the file.
        file.write_all(data.as_bytes())?; // Write the serialized data to the file.
        Ok(())
    }

    // Asynchronously validate an API key with OpenAI's services.
    pub async fn validate_api_key(api_key: &str) -> bool {
        let client = Client::with_config(OpenAIConfig::new().with_api_key(api_key)); // Configure the OpenAI client with the API key.
        match client.models().list().await {
            Ok(_) => true,
            Err(OpenAIError::Reqwest(e)) => {
                send_global_error(crate::error::ShadowrunError::Network(format!(
                    "Please verify your internet connection. Error: {}",
                    e
                )))
                .await;
                false
            }
            _ => false,
        }
    }
}
