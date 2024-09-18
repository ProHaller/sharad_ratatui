// Import necessary modules from the local crate and external crates.
use crate::character::{CharacterSheet, CharacterSheetUpdate};
use serde::{Deserialize, Serialize};

// Define a struct to manage the state of a game session, with serialization and deserialization.
#[derive(Serialize, Deserialize, Clone)]
pub struct GameState {
    pub assistant_id: String,
    pub thread_id: String,
    pub main_character_sheet: Option<CharacterSheet>,
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
            .field("character_sheet", &self.main_character_sheet)
            .finish() // Properly ends the debug struct helper.
    }
}

// Additional implementation for GameState to handle file operations.
impl GameState {
    // Function to load a game state from a specified JSON file.

    pub fn update_character_sheet(&mut self, update: CharacterSheetUpdate) -> Result<(), String> {
        if let Some(ref mut sheet) = self.main_character_sheet {
            sheet.apply_update(update.clone())?;
        }

        if let Some(character) = self.characters.iter_mut().find(|c| {
            c.name
                == self
                    .main_character_sheet
                    .as_ref()
                    .map(|cs| cs.name.clone())
                    .unwrap_or_default()
        }) {
            character.apply_update(update)?;
        }

        Ok(())
    }
}
