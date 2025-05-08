// /character.rs

use derive_more::IntoIterator;
// Import necessary modules from external crates.
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};

use crate::{
    error::{Error, Result},
    ui::descriptions::*,
};

// TODO: Add descriptions everywhere

// Define an enumeration for character races.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, strum_macros::Display)]
pub enum Race {
    Human,
    Elf,
    Dwarf,
    Ork,
    Troll,
}
impl Race {
    pub fn description(&self) -> &str {
        match self {
            Race::Human => HUMAN_DESC,
            Race::Elf => ELF_DESC,
            Race::Dwarf => DWARF_DESC,
            Race::Ork => ORK_DESC,
            Race::Troll => TROLL_DESC,
        }
    }
}

// TODO: refactor that type into a single type struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Magic {
    pub magic: Option<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resonance {
    pub resonance: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attributes {
    pub body: u8,
    pub agility: u8,
    pub strength: u8,
    pub reaction: u8,
    pub willpower: u8,
    pub intuition: u8,
    pub charisma: u8,
    pub logic: u8,
    pub edge: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedAttributes {
    pub initiative: (u8, u8),
    pub limits: Limits,
    pub monitors: Monitors,
    pub essence: Essence,
    pub edge_points: u8,
    pub armor: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    pub physical: u8,
    pub mental: u8,
    pub social: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monitors {
    pub physical: u8,
    pub stun: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Essence {
    pub current: f32,
    pub max: f32,
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
    pub attributes: Attributes,
    pub magic: Magic,
    pub resonance: Resonance,

    // Secondary Attributes
    pub derived_attributes: DerivedAttributes,

    // Skills and Knowledge
    pub skills: Skills,
    pub knowledge_skills: Skill,

    // Economic and Social Information
    #[serde(default)]
    pub nuyen: u32,
    pub lifestyle: String, // TODO: Make this into a struct cf assets/lifestyle.md
    #[serde(default)]
    pub contacts: HashMap<String, Contact>,
    pub qualities: Vec<Quality>,
    pub cyberware: Vec<String>, // TODO: Make this a struct Cyberware
    pub bioware: Vec<String>,   // TODO: Make this a struct Bioware
    #[serde(default)]
    pub inventory: HashMap<String, Item>, // TODO: simplify this data structure to a simple HashMap
    pub matrix_attributes: Option<MatrixAttributes>,
}

pub type Skill = HashMap<String, u8>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skills {
    pub combat: Skill,
    pub physical: Skill,
    pub social: Skill,
    pub technical: Skill,
}

impl IntoIterator for Skills {
    type Item = (String, u8);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut combined: Vec<(String, u8)> = Vec::new();
        combined.extend(self.combat);
        combined.extend(self.physical);
        combined.extend(self.social);
        combined.extend(self.technical);
        combined.into_iter()
    }
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
    pub positive: bool,
    // TODO: Add a description field
    // pub description: String,
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
    pub fn new(builder: CharacterSheetBuilder) -> Self {
        let mut sheet = CharacterSheet {
            name: builder.name,
            race: builder.race,
            gender: builder.gender,
            backstory: builder.backstory,
            main: builder.main,
            attributes: Attributes {
                body: builder.body,
                agility: builder.agility,
                reaction: builder.reaction,
                strength: builder.strength,
                willpower: builder.willpower,
                logic: builder.logic,
                intuition: builder.intuition,
                charisma: builder.charisma,
                edge: builder.edge,
            },
            magic: Magic {
                magic: Some(builder.magic),
            },
            resonance: Resonance {
                resonance: Some(builder.resonance),
            },
            derived_attributes: DerivedAttributes {
                initiative: (builder.reaction + builder.intuition, 1),
                limits: Limits {
                    physical: 1,
                    mental: 1,
                    social: 1,
                },
                monitors: Monitors {
                    physical: 9,
                    stun: 9,
                },
                essence: Essence {
                    current: 6.0,
                    max: 6.0,
                },
                edge_points: 1,
                armor: 0,
            },
            skills: builder.skills,
            knowledge_skills: builder.knowledge_skills,
            nuyen: builder.nuyen,
            lifestyle: "Street".to_string(),
            contacts: builder.contacts,
            qualities: builder.qualities,
            cyberware: Vec::new(),
            bioware: Vec::new(),
            matrix_attributes: None,
            inventory: builder.inventory,
        };

        // Apply race-specific attribute modifiers and update derived attributes.
        sheet.apply_race_modifiers(sheet.race);
        sheet.update_derived_attributes();
        sheet
    }

    // Apply racial modifiers to attributes based on the character's race.
    pub fn apply_race_modifiers(&mut self, race: Race) {
        match race {
            Race::Human => {
                self.attributes.edge = self.attributes.edge.clamp(2, 7);
            }
            Race::Elf => {
                self.attributes.agility = (self.attributes.agility + 1).min(7);
                self.attributes.charisma = (self.attributes.charisma + 2).min(8);
            }
            Race::Dwarf => {
                self.attributes.body = (self.attributes.body + 2).min(8);
                self.attributes.agility = self.attributes.agility.min(5);
                self.attributes.reaction = self.attributes.reaction.min(5);
                self.attributes.strength = (self.attributes.strength + 2).min(8);
                self.attributes.willpower = (self.attributes.willpower + 1).min(7);
            }
            Race::Ork => {
                self.attributes.body = (self.attributes.body + 3).min(9);
                self.attributes.strength = (self.attributes.strength + 2).min(8);
                self.attributes.logic = self.attributes.logic.min(5);
                self.attributes.charisma = self.attributes.charisma.min(5);
            }
            Race::Troll => {
                self.attributes.body = (self.attributes.body + 4).min(10);
                self.attributes.agility = self.attributes.agility.min(5);
                self.attributes.strength = (self.attributes.strength + 4).min(10);
                self.attributes.logic = self.attributes.logic.min(5);
                self.attributes.intuition = self.attributes.intuition.min(5);
                self.attributes.charisma = self.attributes.charisma.min(4);
            }
        }
    }

    // Update derived attributes based on basic and secondary attributes.
    pub fn update_derived_attributes(&mut self) {
        self.derived_attributes.initiative =
            (self.attributes.reaction + self.attributes.intuition, 1);
        self.derived_attributes.monitors.physical = 8 + (self.attributes.body + 1) / 2;
        self.derived_attributes.monitors.stun = 8 + (self.attributes.willpower + 1) / 2;
        self.derived_attributes.limits.physical = ((self.attributes.strength * 2
            + self.attributes.body
            + self.attributes.reaction) as f32
            / 3.0)
            .ceil() as u8;
        self.derived_attributes.limits.mental = ((self.attributes.logic * 2
            + self.attributes.intuition
            + self.attributes.willpower) as f32
            / 3.0)
            .ceil() as u8;
        self.derived_attributes.limits.social = ((self.attributes.charisma * 2
            + self.attributes.willpower
            + self.derived_attributes.essence.current as u8)
            as f32
            / 3.0)
            .ceil() as u8;
    }

    // Retrieve all active skills combined from different skill categories.
    pub fn get_all_active_skills(&self) -> Skill {
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
            "body" => self.attributes.body,
            "agility" => self.attributes.agility,
            "reaction" => self.attributes.reaction,
            "strength" => self.attributes.strength,
            "willpower" => self.attributes.willpower,
            "logic" => self.attributes.logic,
            "intuition" => self.attributes.intuition,
            "charisma" => self.attributes.charisma,
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
            "physical" => self.derived_attributes.limits.physical,
            "mental" => self.derived_attributes.limits.mental,
            "social" => self.derived_attributes.limits.social,
            _ => 0,
        }
    }
}

// Builder for creating CharacterSheet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSheetBuilder {
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
    knowledge_skills: Skill,
    qualities: Vec<Quality>,
    nuyen: u32,
    inventory: HashMap<String, Item>,
    contacts: HashMap<String, Contact>,
}

impl CharacterSheetBuilder {
    pub fn new(name: String, race: Race, gender: String, backstory: String, main: bool) -> Self {
        Self {
            name,
            race,
            gender,
            backstory,
            main,
            body: 1,
            agility: 1,
            reaction: 1,
            strength: 1,
            willpower: 1,
            logic: 1,
            intuition: 1,
            charisma: 1,
            edge: 1,
            magic: 0,
            resonance: 0,
            skills: Skills {
                combat: HashMap::new(),
                physical: HashMap::new(),
                social: HashMap::new(),
                technical: HashMap::new(),
            },
            knowledge_skills: HashMap::new(),
            qualities: vec![],
            nuyen: 0,
            inventory: HashMap::new(),
            contacts: HashMap::new(),
        }
    }

    pub fn body(mut self, body: u8) -> Self {
        self.body = body;
        self
    }

    pub fn agility(mut self, agility: u8) -> Self {
        self.agility = agility;
        self
    }

    pub fn reaction(mut self, reaction: u8) -> Self {
        self.reaction = reaction;
        self
    }

    pub fn strength(mut self, strength: u8) -> Self {
        self.strength = strength;
        self
    }

    pub fn willpower(mut self, willpower: u8) -> Self {
        self.willpower = willpower;
        self
    }

    pub fn logic(mut self, logic: u8) -> Self {
        self.logic = logic;
        self
    }

    pub fn intuition(mut self, intuition: u8) -> Self {
        self.intuition = intuition;
        self
    }

    pub fn charisma(mut self, charisma: u8) -> Self {
        self.charisma = charisma;
        self
    }

    pub fn edge(mut self, edge: u8) -> Self {
        self.edge = edge;
        self
    }

    pub fn magic(mut self, magic: u8) -> Self {
        self.magic = magic;
        self
    }

    pub fn resonance(mut self, resonance: u8) -> Self {
        self.resonance = resonance;
        self
    }

    pub fn skills(mut self, skills: Skills) -> Self {
        self.skills = skills;
        self
    }

    pub fn knowledge_skills(mut self, knowledge_skills: Skill) -> Self {
        self.knowledge_skills = knowledge_skills;
        self
    }

    pub fn qualities(mut self, qualities: Vec<Quality>) -> Self {
        self.qualities = qualities;
        self
    }

    pub fn nuyen(mut self, nuyen: u32) -> Self {
        self.nuyen = nuyen;
        self
    }

    pub fn inventory(mut self, inventory: HashMap<String, Item>) -> Self {
        self.inventory = inventory;
        self
    }

    pub fn contacts(mut self, contacts: HashMap<String, Contact>) -> Self {
        self.contacts = contacts;
        self
    }

    pub fn build(self) -> CharacterSheet {
        CharacterSheet::new(self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UpdateOperation<T> {
    Modify(T),
    Add(T),
    Remove(T),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CharacterSheetUpdate {
    Attribute {
        attribute: String,
        operation: UpdateOperation<CharacterValue>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CharacterValue {
    U8(u8),
    Nuyen(u32),
    String(String),
    Race(Race),
    Skills(Skills),
    HashMapStringU8(Skill),
    VecQuality(Vec<Quality>),
    VecString(Vec<String>),
    HashMapStringItem(HashMap<String, Item>),
    HashMapStringContact(HashMap<String, Contact>),
    OptionMatrixAttributes(Option<MatrixAttributes>),
    OptionU8(Option<u8>),
}

impl CharacterSheet {
    pub fn apply_update(&mut self, update: &CharacterSheetUpdate) -> Result<()> {
        match update {
            CharacterSheetUpdate::Attribute {
                attribute,
                operation,
            } => {
                match operation {
                    UpdateOperation::Modify(value) => self.modify_attribute(attribute, value)?,
                    UpdateOperation::Add(value) => self.add_to_attribute(attribute, value)?,
                    UpdateOperation::Remove(value) => {
                        self.remove_from_attribute(attribute, value)?
                    }
                }
                self.update_derived_attributes();
                Ok(())
            }
        }
    }

    fn modify_attribute(&mut self, attribute: &str, value: &CharacterValue) -> Result<()> {
        match (attribute, value.clone()) {
            ("name", CharacterValue::String(v)) => self.name = v,
            ("race", CharacterValue::Race(v)) => {
                self.race = v;
                self.apply_race_modifiers(self.race);
            }
            ("gender", CharacterValue::String(v)) => self.gender = v,
            ("backstory", CharacterValue::String(v)) => self.backstory = v,
            ("main", CharacterValue::U8(v)) => self.main = v != 0,
            ("body", CharacterValue::U8(v)) => self.attributes.body = v,
            ("agility", CharacterValue::U8(v)) => self.attributes.agility = v,
            ("reaction", CharacterValue::U8(v)) => self.attributes.reaction = v,
            ("strength", CharacterValue::U8(v)) => self.attributes.strength = v,
            ("willpower", CharacterValue::U8(v)) => self.attributes.willpower = v,
            ("logic", CharacterValue::U8(v)) => self.attributes.logic = v,
            ("intuition", CharacterValue::U8(v)) => self.attributes.intuition = v,
            ("charisma", CharacterValue::U8(v)) => self.attributes.charisma = v,
            ("edge", CharacterValue::U8(v)) => self.attributes.edge = v,
            ("magic", CharacterValue::OptionU8(v)) => self.magic.magic = v,
            ("resonance", CharacterValue::OptionU8(v)) => self.resonance.resonance = v,
            ("skills", CharacterValue::Skills(v)) => {
                let Skills {
                    combat: com,
                    physical: phy,
                    social: soc,
                    technical: tech,
                } = v;
                self.skills.combat.extend(com);
                self.skills.physical.extend(phy);
                self.skills.social.extend(soc);
                self.skills.technical.extend(tech);
            }
            ("knowledge_skills", CharacterValue::HashMapStringU8(v)) => {
                self.knowledge_skills.extend(v)
            }
            ("nuyen", CharacterValue::Nuyen(v)) => self.nuyen = v,
            ("lifestyle", CharacterValue::String(v)) => self.lifestyle = v,
            ("contacts", CharacterValue::HashMapStringContact(v)) => self.contacts = v,
            ("qualities", CharacterValue::VecQuality(v)) => self.qualities = v,
            ("cyberware", CharacterValue::VecString(v)) => self.cyberware = v,
            ("bioware", CharacterValue::VecString(v)) => self.bioware = v,
            ("inventory", CharacterValue::HashMapStringItem(v)) => {
                for (key, new_item) in v {
                    if let Some(existing_item) = self.inventory.get_mut(&key) {
                        // Update existing item
                        existing_item.quantity = new_item.quantity;
                        existing_item.description = new_item.description;
                    } else {
                        // Add new item
                        self.inventory.insert(key, new_item);
                    }
                }
            }
            ("matrix_attributes", CharacterValue::OptionMatrixAttributes(v)) => {
                self.matrix_attributes = v
            }
            _ => {
                return Err(format!(
                    "Invalid attribute-value pair for modification: {} {:#?}",
                    attribute, value
                ))
                .map_err(Error::from);
            }
        }
        Ok(())
    }

    fn add_to_attribute(&mut self, attribute: &str, value: &CharacterValue) -> Result<()> {
        match (attribute, value.clone()) {
            ("nuyen", CharacterValue::Nuyen(v)) => self.nuyen = self.nuyen.saturating_add(v),
            ("contacts", CharacterValue::HashMapStringContact(v)) => self.contacts.extend(v),
            ("qualities", CharacterValue::VecQuality(v)) => self.qualities.extend(v),
            ("cyberware", CharacterValue::VecString(v)) => self.cyberware.extend(v),
            ("bioware", CharacterValue::VecString(v)) => self.bioware.extend(v),
            ("inventory", CharacterValue::HashMapStringItem(v)) => {
                for (key, item) in v {
                    if let Some(existing_item) = self.inventory.get_mut(&key) {
                        existing_item.quantity += item.quantity;
                    } else {
                        self.inventory.insert(key, item);
                    }
                }
            }
            _ => {
                return Err(format!(
                    "Invalid attribute-value pair for addition: {} {:#?}",
                    attribute, value
                )
                .into());
            }
        }
        Ok(())
    }

    fn remove_from_attribute(&mut self, attribute: &str, value: &CharacterValue) -> Result<()> {
        match (attribute, value.clone()) {
            ("nuyen", CharacterValue::Nuyen(v)) => self.nuyen = self.nuyen.saturating_sub(v),
            ("contacts", CharacterValue::HashMapStringContact(v)) => {
                for key in v.keys() {
                    self.contacts.remove(key);
                }
            }
            ("qualities", CharacterValue::VecQuality(v)) => {
                self.qualities.retain(|q| !v.contains(q))
            }
            ("cyberware", CharacterValue::VecString(v)) => {
                self.cyberware.retain(|item| !v.contains(item))
            }
            ("bioware", CharacterValue::VecString(v)) => {
                self.bioware.retain(|item| !v.contains(item))
            }
            ("inventory", CharacterValue::HashMapStringItem(v)) => {
                for (key, item) in v {
                    if let Some(existing_item) = self.inventory.get_mut(&key) {
                        if existing_item.quantity <= item.quantity {
                            self.inventory.remove(&key);
                        } else {
                            existing_item.quantity -= item.quantity;
                        }
                    }
                }
            }
            _ => {
                return Err(format!(
                    "Invalid attribute-value pair for removal: {} {:#?}",
                    attribute, value
                )
                .into());
            }
        }
        Ok(())
    }
}
