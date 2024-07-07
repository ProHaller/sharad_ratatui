use crate::character::CharacterSheet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameMessage {
    pub reasoning: String,
    pub narration: String,
    pub character_sheet: Option<CharacterSheet>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_type: MessageType,
    pub content: String, // This will store the raw JSON or error message
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
            .field("message_type", &self.message_type)
            .field("content", &self.content)
            .finish()
    }
}

#[derive(Clone)]
pub enum AIMessage {
    Debug(String),
    Response(String),
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
