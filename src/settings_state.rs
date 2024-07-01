// settings_state.rs

use crate::settings::Settings;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SettingsState {
    pub selected_setting: usize,
    pub selected_options: Vec<usize>,
}

impl SettingsState {
    pub fn from_settings(settings: &Settings) -> Self {
        SettingsState {
            selected_setting: 0,
            selected_options: vec![
                match settings.language.as_str() {
                    "English" => 0,
                    "Français" => 1,
                    "日本語" => 2,
                    _ => 0,
                },
                0, // API Key (always 0 as it's not a toggle)
                if settings.audio_output_enabled { 0 } else { 1 },
                if settings.audio_input_enabled { 0 } else { 1 },
                if settings.debug_mode { 1 } else { 0 },
            ],
        }
    }
}
