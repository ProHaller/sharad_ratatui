// settings_state.rs

use crate::settings::{Language, Settings};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SettingsState {
    pub selected_setting: usize,
    pub selected_options: Vec<usize>,
}

impl SettingsState {
    pub fn from_settings(settings: &Settings) -> Self {
        SettingsState {
            selected_setting: 0,
            selected_options: vec![
                match settings.language {
                    Language::English => 0,
                    Language::French => 1,
                    Language::Japanese => 2,
                    Language::Turkish => 3,
                    // TODO: Add support for custom language
                    _ => 0,
                },
                0, // API Key (always 0 as it's not a toggle)
                match settings.model.as_str() {
                    "gpt-4o-mini" => 0,
                    "gpt-4o" => 1,
                    "o1-mini" => 2,
                    _ => 0,
                },
                if settings.audio_output_enabled { 0 } else { 1 },
                if settings.audio_input_enabled { 0 } else { 1 },
                if settings.debug_mode { 1 } else { 0 },
            ],
        }
    }
}
