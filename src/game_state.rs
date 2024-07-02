use crate::message::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GameState {
    pub assistant_id: String,
    pub thread_id: String,
    #[serde(default)]
    pub message_history: Vec<Message>,
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
}
