// settings.rs

use async_openai::{config::OpenAIConfig, Client};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub language: String,
    pub openai_api_key: Option<String>,
    pub audio_output_enabled: bool,
    pub audio_input_enabled: bool,
    pub debug_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "English".to_string(),
            openai_api_key: None,
            audio_output_enabled: true,
            audio_input_enabled: true,
            debug_mode: false,
        }
    }
}

impl Settings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> io::Result<Self> {
        Self::load_from_file("settings.json")
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to_file("settings.json")
    }

    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let data = fs::read_to_string(path)?;
        let settings = serde_json::from_str(&data)?;
        Ok(settings)
    }

    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        let mut file = fs::File::create(path)?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    pub async fn validate_api_key(api_key: &str) -> bool {
        let client = Client::with_config(OpenAIConfig::new().with_api_key(api_key));
        client.models().list().await.is_ok()
    }
}
