use async_openai::{Client, config::OpenAIConfig, error::OpenAIError};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};
use strum_macros::Display;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub language: Language,
    pub openai_api_key: Option<String>,
    // TODO: Make the model an enum
    pub model: String,
    // TODO: Make the audio an enum
    pub audio_output_enabled: bool,
    pub audio_input_enabled: bool,
    pub debug_mode: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, Display)]
pub enum Language {
    #[default]
    English,
    French,
    Japanese,
    Turkish,
    Custom(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, Display)]
pub enum Model {
    #[default]
    Gpt4oMini,
    Gpt4o,
}

// TODO:  Add a model parameter to change the AI model

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: Language::English,
            openai_api_key: None,
            model: "gpt-4o-mini".to_string(),
            audio_output_enabled: false,
            audio_input_enabled: false,
            debug_mode: true,
        }
    }
}

impl Settings {
    pub fn load() -> io::Result<Self> {
        let home_dir = dir::home_dir().expect("Failed to get home directory");
        let path = home_dir.join("sharad").join("data").join("settings.json");
        Self::load_settings_from_file(path)
    }

    // Load settings from a specified file path.
    pub fn load_settings_from_file(path: PathBuf) -> io::Result<Self> {
        let data = fs::read_to_string(path)?; // Read settings from file.
        let settings = serde_json::from_str(&data)?; // Deserialize JSON data into settings.
        Ok(settings)
    }

    // Save current settings to a specified file path.
    pub fn save_to_file(&self, path: PathBuf) -> io::Result<()> {
        let data = serde_json::to_string_pretty(self)?; // Serialize settings into pretty JSON format.
        if let Some(parent) = path.parent() {
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
            Err(OpenAIError::Reqwest(_)) => false,
            _ => false,
        }
    }
}
