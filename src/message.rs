use crate::ai_response::{GameMessage, SystemMessage, UserMessage};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageType {
    User,
    Game,
    System,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub message_type: MessageType,
}
impl Message {
    pub fn new(content: String, message_type: MessageType) -> Message {
        Message {
            content,
            message_type,
        }
    }
    pub fn from_user_message(user_message: UserMessage) -> Message {
        Message::new(
            format!(
                "Instructions: {}\nPlayer Action: {}",
                user_message.instructions, user_message.player_action
            ),
            MessageType::User,
        )
    }
    pub fn from_game_message(game_message: GameMessage) -> Message {
        Message::new(
            format!(
                "Reasoning: {}\nNarration: {}",
                game_message.reasoning, game_message.narration
            ),
            MessageType::Game,
        )
    }
}
