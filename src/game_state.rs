use crate::character::CharacterSheet;
use crate::message::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GameState {
    pub assistant_id: String,
    pub thread_id: String,
    #[serde(default)]
    pub character_sheet: Option<CharacterSheet>,
    #[serde(default)]
    pub message_history: Vec<Message>,
    pub save_name: String,
}

impl std::fmt::Debug for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameState")
            .field("assistant_id", &self.assistant_id)
            .field("thread_id", &self.thread_id)
            .field("character_sheet", &self.character_sheet)
            .field("message_history", &self.message_history)
            .finish()
    }
}
impl GameState {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let game_state: GameState = serde_json::from_reader(file)?;
        Ok(game_state)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
    pub fn update_character_sheet(&mut self, character_sheet: CharacterSheet) {
        self.character_sheet = Some(character_sheet);
    }
}
