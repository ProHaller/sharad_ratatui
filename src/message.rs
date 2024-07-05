use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageType {
    User,
    Game,
    System,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub instructions: String,
    pub player_action: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameMessage {
    pub reasoning: String,
    pub narration: String,
    // Add other fields for function calling when implemented
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_type: MessageType,
    pub content: String, // This will store the raw JSON or error message
}

impl Message {
    pub fn new(message_type: MessageType, content: String) -> Self {
        Message {
            message_type,
            content,
        }
    }

    pub fn parse_user_message(&self) -> Option<UserMessage> {
        if self.message_type == MessageType::User {
            serde_json::from_str(&self.content).ok()
        } else {
            None
        }
    }

    pub fn parse_game_message(&self) -> Option<GameMessage> {
        if self.message_type == MessageType::Game {
            serde_json::from_str(&self.content).ok()
        } else {
            None
        }
    }
}
