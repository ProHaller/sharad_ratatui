// Import necessary modules from the local crate and external crates.
use crate::character::{CharacterSheet, CharacterSheetUpdate};
use crate::message::Message;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;

// Define a struct to manage the state of a game session, with serialization and deserialization.
#[derive(Serialize, Deserialize, Clone)]
pub struct GameState {
    pub assistant_id: String,
    pub thread_id: String,
    pub character_sheet: Option<CharacterSheet>,
    pub message_history: Vec<Message>,
    pub save_name: String,
    pub characters: Vec<CharacterSheet>,
}

// Implement the Debug trait manually to control what information is shown when debug printed.
impl std::fmt::Debug for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Custom format to display the structure's fields.
        f.debug_struct("GameState")
            .field("assistant_id", &self.assistant_id)
            .field("thread_id", &self.thread_id)
            .field("character_sheet", &self.character_sheet)
            .field("message_history", &self.message_history)
            .finish() // Properly ends the debug struct helper.
    }
}

// Additional implementation for GameState to handle file operations.
impl GameState {
    // Function to load a game state from a specified JSON file.
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let game_state: GameState = serde_json::from_reader(file)?;

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("sharad_debug.log")
        {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(file, "[{}] load_from_file:\n {:?}", timestamp, game_state);
        }
        Ok(game_state)
    }

    // Function to save the current game state to a specified file in JSON format.
    pub fn save_to_file(&self, path: &str) -> Result<(), std::io::Error> {
        let file = std::fs::File::create(path)?; // Create or overwrite the file at the specified path.
        serde_json::to_writer_pretty(file, self)?; // Serialize the GameState into JSON and write to the file.
        Ok(()) // Return success if the file is written without errors.
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let save_dir = "./data/save";
        std::fs::create_dir_all(save_dir)?;
        let save_path = format!("{}/{}.json", save_dir, self.save_name);
        let serialized = serde_json::to_string_pretty(self)?;
        std::fs::write(save_path, serialized)?;
        Ok(())
    }

    pub fn update_character_sheet(&mut self, update: CharacterSheetUpdate) -> Result<(), String> {
        if let Some(ref mut sheet) = self.character_sheet {
            sheet.apply_update(update.clone())?;
        }

        if let Some(character) = self.characters.iter_mut().find(|c| {
            c.name
                == self
                    .character_sheet
                    .as_ref()
                    .map(|cs| cs.name.clone())
                    .unwrap_or_default()
        }) {
            character.apply_update(update)?;
        }

        Ok(())
    }
}
