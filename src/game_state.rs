use std::path::PathBuf;

// Import necessary modules from the local crate and external crates.
use crate::character::CharacterSheet;
use serde::{Deserialize, Serialize};

// Define a struct to manage the state of a game session, with serialization and deserialization.
#[derive(Serialize, Deserialize, Clone)]
pub struct GameState {
    pub assistant_id: String,
    pub thread_id: String,
    pub save_name: String,
    pub characters: Vec<CharacterSheet>,
    pub save_path: Option<PathBuf>,
    pub main_character_sheet: Option<CharacterSheet>,
    pub image_path: Option<PathBuf>,
}
impl GameState {
    pub fn new(assistant_id: String, thread_id: String, save_name: String) -> Self {
        Self {
            assistant_id,
            thread_id,
            save_name,
            characters: Vec::new(),
            save_path: None,
            main_character_sheet: None,
            image_path: None,
        }
    }
}

// Implement the Debug trait manually to control what information is shown when debug printed.
impl std::fmt::Debug for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Custom format to display the structure's fields.
        f.debug_struct("GameState")
            .field("assistant_id", &self.assistant_id)
            .field("thread_id", &self.thread_id)
            .field("character_sheet", &self.main_character_sheet)
            .field("image_path", &self.image_path)
            .finish() // Properly ends the debug struct helper.
    }
}
