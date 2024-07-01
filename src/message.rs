// message.rs

#[derive(Clone, PartialEq)]
pub enum MessageType {
    User,
    Game,
    System,
}

#[derive(Clone)]
pub struct Message {
    pub content: String,
    pub message_type: MessageType,
}
