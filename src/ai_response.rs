// src/ai_response.rs
// Import necessary modules and structs from other parts of the application or crates.
use crate::character::CharacterSheet;
use serde::{Deserialize, Serialize};

// Define a structure for user-generated messages with fields for instructions and player actions.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserMessage {
    pub instructions: String,  // Instructions to the player or game master.
    pub player_action: String, // Specific action taken by the player.
}

// Define a structure for system-generated messages.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemMessage {
    pub message: String, // Content of the system message.
}

// Define a structure for messages generated within the game's mechanics.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameMessage {
    pub crunch: String, // Explanation of why this message is generated.
    pub fluff: String,  // Narrative content for the game's story.
    pub character_sheet: Option<CharacterSheet>, // Optional character sheet if relevant to the message.
}

// Implementation block for UserMessage with a constructor method.
impl UserMessage {
    // Constructor for creating a new UserMessage instance.
    pub fn new(instructions: String, player_action: String) -> Self {
        UserMessage {
            instructions,
            player_action,
        }
    }
}

// Function to create a new UserMessage with predefined instructions.
pub fn create_user_message(language: &str, player_action: &str) -> UserMessage {
    UserMessage::new(
        // Long string for default instructions to act as a professional Game Master.
        format!("Act as the Game Master in a Shadowrun table top role-playing game. Allow the player to attempt one action at a time without providing choices. For actions involving multiple steps or failure points, require the player to choose a course of action at each step. Make sure the story keeps progressing by leading the story line. Keep the story going as a good Game Master, never let the tension fall down. Write your response in valid JSON in the following language: {}", language).to_string(),
        player_action.to_string(), // Convert the input action to a String and pass it to the new UserMessage.
    )
}
