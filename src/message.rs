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
