// src/ai_response.rs
use crate::character::CharacterSheet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserMessage {
    pub instructions: String,
    pub player_action: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemMessage {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameMessage {
    pub reasoning: String,
    pub narration: String,
    pub character_sheet: Option<CharacterSheet>,
}

impl UserMessage {
    pub fn new(instructions: String, player_action: String) -> Self {
        UserMessage {
            instructions,
            player_action,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_ai_format(&self) -> String {
        match self.to_json() {
            Ok(json) => json,
            Err(_) => format!(
                r#"{{"instructions": "{}", "player_action": "{}"}}"#,
                self.instructions, self.player_action
            ),
        }
    }
}

impl GameMessage {
    pub fn new(reasoning: String, narration: String) -> Self {
        GameMessage {
            reasoning,
            narration,
            character_sheet: None,
        }
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl SystemMessage {
    pub fn new(message: String) -> Self {
        SystemMessage { message }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

pub fn create_user_message(player_action: &str) -> UserMessage {
    UserMessage::new(
        "Act as a professional Game Master in a role-playing game. Evaluate the probability of success for each intended player action and roll the dice when pertinent. If an action falls outside the player's skills and capabilities, make them fail and face the consequences, which could include death. Allow the player to attempt one action at a time without providing choices. Do not allow the player to summon anything that was not previously introduced unless it is perfectly innocuous. For actions involving multiple steps or failure points, require the player to choose a course of action at each step. Write your reasoning and the results of the dice roll in a JSON ".to_string(),
        player_action.to_string(),
    )
}
