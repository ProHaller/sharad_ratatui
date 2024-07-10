use crate::game_state::GameState;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct DiceRollRequest {
    character_name: String,
    attribute: String,
    skill: String,
    limit_type: String,
    threshold: Option<u8>,
    edge_action: Option<String>,
    extra_dice: Option<u8>,
}

#[derive(Debug, Serialize)]
pub struct DiceRollResponse {
    pub hits: u8,
    pub glitch: bool,
    pub critical_glitch: bool,
    pub critical_success: bool,
    pub dice_results: Vec<u8>,
    pub success: bool,
}

pub fn perform_dice_roll(
    request: DiceRollRequest,
    game_state: &GameState,
) -> Result<DiceRollResponse, String> {
    // Find the character in the game state
    let character = game_state
        .characters
        .iter()
        .find(|c| c.name == request.character_name)
        .ok_or_else(|| format!("Character '{}' not found", request.character_name))?;

    // Get the dice pool
    let dice_pool = character.get_dice_pool(&request.attribute, &request.skill);

    // Get the limit
    let limit = Some(character.get_limit(&request.limit_type));

    // Parse the edge action
    let edge_action = match request.edge_action.as_deref() {
        Some("RerollFailures") => Some(EdgeAction::RerollFailures),
        Some("AddExtraDice") => request.extra_dice.map(EdgeAction::AddExtraDice),
        Some("PushTheLimit") => Some(EdgeAction::PushTheLimit),
        Some(_) => return Err("Invalid edge action".to_string()),
        None => None,
    };

    // Perform the dice roll
    let roll_result = dice_roll(dice_pool, limit, request.threshold, edge_action);

    // Determine if the roll was successful
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
pub struct DiceRoll {
    pub hits: u8,
    pub glitch: bool,
    pub critical_glitch: bool,
    pub critical_success: bool,
    pub dice_results: Vec<u8>,
}

pub fn dice_roll(
    dice_pool: u8,
    limit: Option<u8>,
    threshold: Option<u8>,
    edge_action: Option<EdgeAction>,
) -> DiceRoll {
    let mut rng = rand::thread_rng();
    let mut dice_results = Vec::new();
    let mut hits = 0;
    let mut ones = 0;

    for _ in 0..dice_pool {
        let mut die_result = roll_die(&mut rng);
        dice_results.push(die_result);

        // Implement "Rule of Six"
        while die_result == 6 {
            die_result = roll_die(&mut rng);
            dice_results.push(die_result);
        }

        if die_result >= 5 {
            hits += 1;
        } else if die_result == 1 {
            ones += 1;
        }
    }

    // Apply Edge action if any
    if let Some(edge_action) = edge_action {
        apply_edge_action(
            &mut dice_results,
            &mut hits,
            &mut ones,
            edge_action,
            &mut rng,
        );
    }

    // Apply limit if specified
    if let Some(lim) = limit {
        hits = hits.min(lim);
    }

    let glitch = ones > dice_pool as usize / 2;
    let critical_glitch = glitch && hits == 0;

    // Determine if it's a critical success
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

fn roll_die(rng: &mut impl Rng) -> u8 {
    rng.gen_range(1..=6)
}

pub enum EdgeAction {
    RerollFailures,
    AddExtraDice(u8),
    PushTheLimit,
    // Add more Edge actions as needed
}

fn apply_edge_action(
    dice_results: &mut Vec<u8>,
    hits: &mut u8,
    ones: &mut usize,
    edge_action: EdgeAction,
    rng: &mut impl Rng,
) {
    match edge_action {
        EdgeAction::RerollFailures => {
            for die in dice_results.iter_mut() {
                if *die < 5 {
                    if *die == 1 {
                        *ones -= 1;
                    }
                    *die = roll_die(rng);
                    if *die >= 5 {
                        *hits += 1;
                    } else if *die == 1 {
                        *ones += 1;
                    }
                }
            }
        }
        EdgeAction::AddExtraDice(extra) => {
            for _ in 0..extra {
                let mut die_result = roll_die(rng);
                dice_results.push(die_result);

                while die_result == 6 {
                    die_result = roll_die(rng);
                    dice_results.push(die_result);
                }

                if die_result >= 5 {
                    *hits += 1;
                } else if die_result == 1 {
                    *ones += 1;
                }
            }
        }
        EdgeAction::PushTheLimit => {
            // No action needed here, as limit is handled in the main function
        }
    }
}
