// Import necessary modules from external crates.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// Define an enumeration for character races in a role-playing game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Race {
    Human,
    Elf,
    Dwarf,
    Ork,
    Troll,
}

// Implement the Display trait for the Race enum to allow for easier printing.
impl fmt::Display for Race {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Race::Human => write!(f, "Human"),
            Race::Elf => write!(f, "Elf"),
            Race::Dwarf => write!(f, "Dwarf"),
            Race::Ork => write!(f, "Ork"),
            Race::Troll => write!(f, "Troll"),
        }
    }
}

// Define a structure representing a character's information sheet in a role-playing game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSheet {
    // Personal Information
    pub name: String,
    pub race: Race,
    pub gender: String,
    pub backstory: String,
    pub main: bool,

    // Basic Attributes
    pub body: u8,
    pub agility: u8,
    pub reaction: u8,
    pub strength: u8,
    pub willpower: u8,
    pub logic: u8,
    pub intuition: u8,
    pub charisma: u8,
    pub edge: u8,
    pub magic: Option<u8>,
    pub resonance: Option<u8>,

    // Secondary Attributes
    pub initiative: (u8, u8),
    pub essence: f32,
    pub edge_points: u8,
    pub physical_monitor: u8,
    pub stun_monitor: u8,

    // Derived Attributes
    pub armor: u8,
    pub physical_limit: u8,
    pub mental_limit: u8,
    pub social_limit: u8,

    // Skills and Knowledge
    pub skills: Skills,
    pub knowledge_skills: HashMap<String, u8>,

    // Economic and Social Information
    #[serde(default)]
    pub nuyen: u32,
    pub lifestyle: String,
    #[serde(default)]
    pub contacts: HashMap<String, Contact>,
    pub qualities: Vec<Quality>,
    pub cyberware: Vec<String>,
    pub bioware: Vec<String>,
    #[serde(default)]
    pub inventory: HashMap<String, Item>,
    pub matrix_attributes: Option<MatrixAttributes>,
}

// Define a structure for categorizing different skills a character may have.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skills {
    pub combat: HashMap<String, u8>,
    pub physical: HashMap<String, u8>,
    pub social: HashMap<String, u8>,
    pub technical: HashMap<String, u8>,
}

// Define a structure for items that can be part of a character's inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub quantity: u32,
    pub description: String,
}

// Define a structure for contacts within the game, representing relationships and connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    pub description: String,
    pub loyalty: u8,
    pub connection: u8,
}

// Define a structure for character qualities, representing traits or special abilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Quality {
    pub name: String,
    pub positive: bool, // Indicates if the quality is advantageous.
}

// Define a structure for matrix attributes, applicable if the character interacts with virtual environments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixAttributes {
    pub attack: u8,
    pub sleaze: u8,
    pub data_processing: u8,
    pub firewall: u8,
}

// Implementation of methods for the CharacterSheet struct.
impl CharacterSheet {
    // Constructor for creating a new character sheet.
    pub fn new(
        name: String,
        race: Race,
        gender: String,
        backstory: String,
        main: bool,
        body: u8,
        agility: u8,
        reaction: u8,
        strength: u8,
        willpower: u8,
        logic: u8,
        intuition: u8,
        charisma: u8,
        edge: u8,
        magic: u8,
        resonance: u8,
        skills: Skills,
        qualities: Vec<Quality>,
        nuyen: u32,
    ) -> Self {
        let mut sheet = CharacterSheet {
            name,
            race: race.clone(),
            gender,
            backstory,
            main,
            body,
            agility,
            reaction,
            strength,
            willpower,
            logic,
            intuition,
            charisma,
            edge,
            magic: Some(magic),
            resonance: Some(resonance),
            initiative: (0, 1),
            essence: 6.0,
            edge_points: 1,
            physical_monitor: 9,
            stun_monitor: 9,
            armor: 0,
            physical_limit: 1,
            mental_limit: 1,
            social_limit: 1,
            skills,
            knowledge_skills: HashMap::new(),
            nuyen,
            lifestyle: "Street".to_string(),
            contacts: HashMap::new(),
            qualities,
            cyberware: Vec::new(),
            bioware: Vec::new(),
            inventory: HashMap::new(),
            matrix_attributes: None,
        };

        // Apply race-specific attribute modifiers and update derived attributes.
        sheet.apply_race_modifiers(sheet.race.clone());
        sheet.update_derived_attributes();
        sheet
    }

    // Apply racial modifiers to attributes based on the character's race.
    pub fn apply_race_modifiers(&mut self, race: Race) {
        match race {
            Race::Human => {
                self.edge = self.edge.max(2).min(7);
            }
            Race::Elf => {
                self.agility = (self.agility + 1).min(7);
                self.charisma = (self.charisma + 2).min(8);
            }
            Race::Dwarf => {
                self.body = (self.body + 2).min(8);
                self.agility = self.agility.min(5);
                self.reaction = self.reaction.min(5);
                self.strength = (self.strength + 2).min(8);
                self.willpower = (self.willpower + 1).min(7);
            }
            Race::Ork => {
                self.body = (self.body + 3).min(9);
                self.strength = (self.strength + 2).min(8);
                self.logic = self.logic.min(5);
                self.charisma = self.charisma.min(5);
            }
            Race::Troll => {
                self.body = (self.body + 4).min(10);
                self.agility = self.agility.min(5);
                self.strength = (self.strength + 4).min(10);
                self.logic = self.logic.min(5);
                self.intuition = self.intuition.min(5);
                self.charisma = self.charisma.min(4);
            }
        }
    }

    // Update derived attributes based on basic and secondary attributes.
    pub fn update_derived_attributes(&mut self) {
        self.initiative = (self.reaction + self.intuition, 1);
        self.physical_monitor = 8 + (self.body + 1) / 2;
        self.stun_monitor = 8 + (self.willpower + 1) / 2;
        self.physical_limit =
            ((self.strength * 2 + self.body + self.reaction) as f32 / 3.0).ceil() as u8;
        self.mental_limit =
            ((self.logic * 2 + self.intuition + self.willpower) as f32 / 3.0).ceil() as u8;
        self.social_limit =
            ((self.charisma * 2 + self.willpower + self.essence as u8) as f32 / 3.0).ceil() as u8;
    }

    // Retrieve all active skills combined from different skill categories.
    pub fn get_all_active_skills(&self) -> HashMap<String, u8> {
        let mut all_skills = HashMap::new();
        all_skills.extend(self.skills.combat.clone());
        all_skills.extend(self.skills.physical.clone());
        all_skills.extend(self.skills.social.clone());
        all_skills.extend(self.skills.technical.clone());
        all_skills
    }

    // Calculate the dice pool for an action based on attribute and skill levels.
    pub fn get_dice_pool(&self, attribute: &str, skill: &str) -> u8 {
        let attribute_value = match attribute.to_lowercase().as_str() {
            "body" => self.body,
            "agility" => self.agility,
            "reaction" => self.reaction,
            "strength" => self.strength,
            "willpower" => self.willpower,
            "logic" => self.logic,
            "intuition" => self.intuition,
            "charisma" => self.charisma,
            _ => 0,
        };

        let skill_value = self
            .get_all_active_skills()
            .get(skill)
            .cloned()
            .unwrap_or(0);

        attribute_value + skill_value
    }

    // Get the maximum limit for an action based on the type of limit (physical, mental, social).
    pub fn get_limit(&self, limit_type: &str) -> u8 {
        match limit_type.to_lowercase().as_str() {
            "physical" => self.physical_limit,
            "mental" => self.mental_limit,
            "social" => self.social_limit,
            _ => 0,
        }
    }

    pub fn apply_update(&mut self, update: CharacterSheetUpdate) -> Result<(), String> {
        match update {
            CharacterSheetUpdate::UpdateNuyen { amount } => {
                if amount >= 0 {
                    self.nuyen = self.nuyen.saturating_add(amount as u32);
                } else {
                    self.nuyen = self.nuyen.saturating_sub(amount.abs() as u32);
                }
            }
            CharacterSheetUpdate::AddContact {
                name,
                description,
                loyalty,
                connection,
            } => {
                self.contacts.insert(
                    name.clone(),
                    Contact {
                        name,
                        description,
                        loyalty,
                        connection,
                    },
                );
            }
            CharacterSheetUpdate::RemoveContact { name } => {
                self.contacts.remove(&name);
            }
            CharacterSheetUpdate::AddInventoryItem {
                name,
                quantity,
                description,
            } => {
                self.inventory
                    .entry(name.clone())
                    .and_modify(|item| item.quantity += quantity)
                    .or_insert(Item {
                        name,
                        quantity,
                        description,
                    });
            }
            CharacterSheetUpdate::RemoveInventoryItem { name } => {
                if let Some(item) = self.inventory.get_mut(&name) {
                    item.quantity = item.quantity.saturating_sub(1);
                    if item.quantity == 0 {
                        self.inventory.remove(&name);
                    }
                }
            }
            CharacterSheetUpdate::AddCyberware { name } => {
                self.cyberware.push(name);
            }
            CharacterSheetUpdate::RemoveCyberware { name } => {
                self.cyberware.retain(|c| c != &name);
            }
            CharacterSheetUpdate::AddBioware { name } => {
                self.bioware.push(name);
            }
            CharacterSheetUpdate::RemoveBioware { name } => {
                self.bioware.retain(|b| b != &name);
            }
            CharacterSheetUpdate::UpdateAttribute { attribute, value } => {
                match attribute.to_lowercase().as_str() {
                    "body" => self.body = value,
                    "agility" => self.agility = value,
                    "reaction" => self.reaction = value,
                    "strength" => self.strength = value,
                    "willpower" => self.willpower = value,
                    "logic" => self.logic = value,
                    "intuition" => self.intuition = value,
                    "charisma" => self.charisma = value,
                    "edge" => self.edge = value,
                    _ => return Err(format!("Invalid attribute: {}", attribute)),
                }
                self.update_derived_attributes();
            }
            CharacterSheetUpdate::UpdateSkill {
                category,
                skill,
                value,
            } => {
                let skill_map = match category.to_lowercase().as_str() {
                    "combat" => &mut self.skills.combat,
                    "physical" => &mut self.skills.physical,
                    "social" => &mut self.skills.social,
                    "technical" => &mut self.skills.technical,
                    _ => return Err(format!("Invalid skill category: {}", category)),
                };
                skill_map.insert(skill, value);
            }
            CharacterSheetUpdate::AddQuality { name, positive } => {
                self.qualities.push(Quality { name, positive });
            }
            CharacterSheetUpdate::RemoveQuality { name } => {
                self.qualities.retain(|c| c.name != name);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CharacterSheetUpdate {
    UpdateNuyen {
        amount: i32,
    },
    AddContact {
        name: String,
        description: String,
        loyalty: u8,
        connection: u8,
    },
    RemoveContact {
        name: String,
    },
    AddInventoryItem {
        name: String,
        quantity: u32,
        description: String,
    },
    RemoveInventoryItem {
        name: String,
    },
    AddCyberware {
        name: String,
    },
    RemoveCyberware {
        name: String,
    },
    AddBioware {
        name: String,
    },
    RemoveBioware {
        name: String,
    },
    UpdateAttribute {
        attribute: String,
        value: u8,
    },
    UpdateSkill {
        category: String,
        skill: String,
        value: u8,
    },
    AddQuality {
        name: String,
        positive: bool,
    },
    RemoveQuality {
        name: String,
    },
}
