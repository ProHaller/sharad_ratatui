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
pub fn create_user_message(player_action: &str) -> UserMessage {
    UserMessage::new(
        // Long string for default instructions to act as a professional Game Master.
        "Act as a professional Game Master in a role-playing game. Evaluate the probability of success for each intended player action and roll the dice when pertinent. If an action falls outside the player's skills and capabilities, make them fail and face the consequences, which could include death. Allow the player to attempt one action at a time without providing choices. Do not allow the player to summon anything that was not previously introduced unless it is perfectly innocuous. For actions involving multiple steps or failure points, require the player to choose a course of action at each step. Write your crunch and the results of the dice roll in a JSON ".to_string(),
        player_action.to_string(), // Convert the input action to a String and pass it to the new UserMessage.
    )
}
