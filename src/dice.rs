// Import required modules and crates.
use crate::game_state::GameState;
use rand::Rng; // RNG utilities from the rand crate for generating random numbers.
use serde::{Deserialize, Serialize}; // Serialization utilities for struct serialization.

// Structure to handle the request for a dice roll.
#[derive(Deserialize)]
pub struct DiceRollRequest {
    character_name: String,      // Name of the character making the roll.
    attribute: String,           // The attribute involved in the dice roll.
    skill: String,               // The skill involved in the dice roll.
    limit_type: String,          // The type of limit (e.g., physical, mental) applied to the roll.
    threshold: Option<u8>,       // Optional threshold for determining success.
    edge_action: Option<String>, // Optional action that uses "edge" to affect the roll.
    extra_dice: Option<u8>,      // Optional number of extra dice to roll.
}

// Structure to encapsulate the response after a dice roll.
#[derive(Debug, Serialize)]
pub struct DiceRollResponse {
    pub dice_results: Vec<u8>,  // Results of each die rolled.
    pub hits: u8,               // Number of successful hits.
    pub success: bool,          // Whether the roll was overall a success.
    pub glitch: bool,           // Whether a glitch occurred.
    pub critical_glitch: bool,  // Whether a critical glitch occurred.
    pub critical_success: bool, // Whether a critical success was achieved.
}

// Function to perform a dice roll based on a request and game state.
pub fn perform_dice_roll(
    request: DiceRollRequest,
    game_state: &GameState,
) -> Result<DiceRollResponse, String> {
    // Find the character by name from the game state.
    let character = game_state
        .characters
        .iter()
        .find(|c| c.name == request.character_name)
        .ok_or_else(|| format!("Character '{}' not found", request.character_name))?;

    // Calculate the total dice pool from character's attributes and skills.
    let dice_pool = character.get_dice_pool(&request.attribute, &request.skill);

    // Get the applicable limit for the dice roll from the character's stats.
    let limit = Some(character.get_limit(&request.limit_type));

    // Parse the optional edge action.
    let edge_action = match request.edge_action.as_deref() {
        Some("RerollFailures") => Some(EdgeAction::RerollFailures),
        Some("AddExtraDice") => request.extra_dice.map(EdgeAction::AddExtraDice),
        Some("PushTheLimit") => Some(EdgeAction::PushTheLimit),
        Some(_) => return Err("Invalid edge action".to_string()),
        None => None,
    };

    // Execute the dice roll with the calculated parameters.
    let roll_result = dice_roll(dice_pool, limit, request.threshold, edge_action);

    // Determine if the roll met the success criteria.
    let success = match request.threshold {
        Some(threshold) => roll_result.hits >= threshold,
        None => roll_result.hits > 0,
    };

    Ok(DiceRollResponse {
        hits: roll_result.hits,
        glitch: roll_result.glitch,
        critical_glitch: roll_result.critical_glitch,
        critical_success: roll_result.critical_success,
        dice_results: roll_result.dice_results,
        success,
    })
}

// Structure to hold the results of a dice roll.
pub struct DiceRoll {
    pub hits: u8,
    pub glitch: bool,
    pub critical_glitch: bool,
    pub critical_success: bool,
    pub dice_results: Vec<u8>,
}

// Function to execute the dice roll logic.
pub fn dice_roll(
    dice_pool: u8,
    limit: Option<u8>,
    threshold: Option<u8>,
    edge_action: Option<EdgeAction>,
) -> DiceRoll {
    let mut rng = rand::rng(); // Random number generator.
    let mut dice_results = Vec::new(); // Store results of each die roll.
    let mut hits = 0; // Count of successful hits (dice results of 5 or 6).
    let mut ones = 0; // Count of dice results that are 1, which might indicate a glitch.

    // Roll the dice as per the dice pool count.
    for _ in 0..dice_pool {
        let mut die_result = roll_die(&mut rng); // Roll a single die.
        dice_results.push(die_result);

        // Implement "Rule of Six" where a roll of 6 allows re-rolling.
        while die_result == 6 {
            hits += 1; // Count hits from the dice.
            die_result = roll_die(&mut rng);
            dice_results.push(die_result);
        }

        if die_result >= 5 {
            hits += 1; // A roll of 5 or 6 counts as a hit.
        } else if die_result == 1 {
            ones += 1; // Count ones to check for glitches.
        }
    }

    // Apply any edge actions that may alter the outcome of the roll.
    if let Some(edge_action) = edge_action {
        apply_edge_action(
            &mut dice_results,
            &mut hits,
            &mut ones,
            edge_action,
            &mut rng,
        );
    }

    // Apply the limit to the number of hits if specified.
    if let Some(lim) = limit {
        hits = hits.min(lim);
    }

    // Determine if a glitch or a critical glitch occurred.
    let glitch = ones > dice_pool as usize / 2;
    let critical_glitch = glitch && hits == 0;

    // Check for critical success if a threshold is specified.
    let critical_success = match threshold {
        Some(t) => hits >= t * 2,
        None => false,
    };

    DiceRoll {
        hits,
        glitch,
        critical_glitch,
        critical_success,
        dice_results,
    }
}

// Helper function to roll a single die.
fn roll_die(rng: &mut impl Rng) -> u8 {
    rng.random_range(1..=6)
}

// Enum to represent possible edge actions during a dice roll.
pub enum EdgeAction {
    RerollFailures,
    AddExtraDice(u8),
    PushTheLimit, // Ignore limits on the dice roll.
                  // Additional edge actions could be added here.
}

// Function to apply an edge action during a dice roll.
fn apply_edge_action(
    dice_results: &mut Vec<u8>,
    hits: &mut u8,
    ones: &mut usize,
    edge_action: EdgeAction,
    rng: &mut impl Rng,
) {
    match edge_action {
        EdgeAction::RerollFailures => {
            // Reroll all dice that failed to hit (less than 5).
            for die in dice_results.iter_mut() {
                if *die < 5 {
                    if *die == 1 {
                        *ones -= 1; // Adjust the count of ones if rerolling a one.
                    }
                    *die = roll_die(rng); // Reroll the die.
                    if *die >= 5 {
                        *hits += 1; // Increment hits if the reroll is successful.
                    } else if *die == 1 {
                        *ones += 1; // Increment ones if the reroll results in a one.
                    }
                }
            }
        }
        EdgeAction::AddExtraDice(extra) => {
            // Roll additional dice specified by the edge action.
            for _ in 0..extra {
                let mut die_result = roll_die(rng);
                dice_results.push(die_result);

                while die_result == 6 {
                    *hits += 1; // Count hits from the additional dice.
                    die_result = roll_die(rng);
                    dice_results.push(die_result);
                }

                if die_result >= 5 {
                    *hits += 1; // Count hits from the additional dice.
                } else if die_result == 1 {
                    *ones += 1; // Count ones from the additional dice.
                }
            }
        }
        EdgeAction::PushTheLimit => {
            // This edge action typically affects limit handling, which is considered in the main dice roll function.
        }
    }
}
