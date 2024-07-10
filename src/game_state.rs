// Import necessary modules from the local crate and external crates.
use crate::character::CharacterSheet;
use crate::message::Message;
use serde::{Deserialize, Serialize};

// Define a struct to manage the state of a game session, with serialization and deserialization.
#[derive(Serialize, Deserialize, Clone)]
pub struct GameState {
    pub assistant_id: String, // Identifier for the assistant managing the game.
    pub thread_id: String,    // Identifier for the specific game thread or session.
    #[serde(default)] // Use default value if not provided during deserialization.
    pub character_sheet: Option<CharacterSheet>, // Optional character sheet, if applicable to the game session.
    #[serde(default)] // Ensure that an empty vector is used if not provided.
    pub message_history: Vec<Message>, // History of messages within the game session.
    pub save_name: String, // The name under which this game state is saved.
    pub characters: Vec<CharacterSheet>, // List of character sheets for all characters in the game.
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
        let file = std::fs::File::open(path)?; // Attempt to open the specified file.
        let game_state: GameState = serde_json::from_reader(file)?; // Deserialize the JSON into a GameState object.
        Ok(game_state) // Return the deserialized game state.
    }

    // Function to save the current game state to a specified file in JSON format.
    pub fn save_to_file(&self, path: &str) -> Result<(), std::io::Error> {
        let file = std::fs::File::create(path)?; // Create or overwrite the file at the specified path.
        serde_json::to_writer_pretty(file, self)?; // Serialize the GameState into JSON and write to the file.
        Ok(()) // Return success if the file is written without errors.
    }
}
