// Import necessary modules from external crates.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// Define an enumeration for character races.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub fn new(builder: CharacterSheetBuilder) -> Self {
        let mut sheet = CharacterSheet {
            name: builder.name,
            race: builder.race.clone(),
            gender: builder.gender,
            backstory: builder.backstory,
            main: builder.main,
            body: builder.body,
            agility: builder.agility,
            reaction: builder.reaction,
            strength: builder.strength,
            willpower: builder.willpower,
            logic: builder.logic,
            intuition: builder.intuition,
            charisma: builder.charisma,
            edge: builder.edge,
            magic: Some(builder.magic),
            resonance: Some(builder.resonance),
            initiative: (0, 1),
            essence: 6.0,
            edge_points: 1,
            physical_monitor: 9,
            stun_monitor: 9,
            armor: 0,
            physical_limit: 1,
            mental_limit: 1,
            social_limit: 1,
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
        sheet.apply_race_modifiers(sheet.race.clone());
        sheet.update_derived_attributes();
        sheet
    }

    // Apply racial modifiers to attributes based on the character's race.
    pub fn apply_race_modifiers(&mut self, race: Race) {
        match race {
            Race::Human => {
                self.edge = self.edge.clamp(2, 7);
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
    knowledge_skills: HashMap<String, u8>,
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

    pub fn knowledge_skills(mut self, knowledge_skills: HashMap<String, u8>) -> Self {
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
    UpdateAttribute {
        attribute: String,
        operation: UpdateOperation<Value>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    U8(u8),
    U32(u32),
    String(String),
    Race(Race),
    Skills(Skills),
    HashMapStringU8(HashMap<String, u8>),
    VecQuality(Vec<Quality>),
    VecString(Vec<String>),
    HashMapStringItem(HashMap<String, Item>),
    HashMapStringContact(HashMap<String, Contact>),
    OptionMatrixAttributes(Option<MatrixAttributes>),
    OptionU8(Option<u8>),
}

impl CharacterSheet {
    pub fn apply_update(&mut self, update: CharacterSheetUpdate) -> Result<(), String> {
        match update {
            CharacterSheetUpdate::UpdateAttribute {
                attribute,
                operation,
            } => {
                match operation {
                    UpdateOperation::Modify(value) => self.modify_attribute(&attribute, value)?,
                    UpdateOperation::Add(value) => self.add_to_attribute(&attribute, value)?,
                    UpdateOperation::Remove(value) => {
                        self.remove_from_attribute(&attribute, value)?
                    }
                }
                self.update_derived_attributes();
                Ok(())
            }
        }
    }

    fn modify_attribute(&mut self, attribute: &str, value: Value) -> Result<(), String> {
        match (attribute, value.clone()) {
            ("name", Value::String(v)) => self.name = v,
            ("race", Value::Race(v)) => {
                self.race = v;
                self.apply_race_modifiers(self.race.clone());
            }
            ("gender", Value::String(v)) => self.gender = v,
            ("backstory", Value::String(v)) => self.backstory = v,
            ("main", Value::U8(v)) => self.main = v != 0,
            ("body", Value::U8(v)) => self.body = v,
            ("agility", Value::U8(v)) => self.agility = v,
            ("reaction", Value::U8(v)) => self.reaction = v,
            ("strength", Value::U8(v)) => self.strength = v,
            ("willpower", Value::U8(v)) => self.willpower = v,
            ("logic", Value::U8(v)) => self.logic = v,
            ("intuition", Value::U8(v)) => self.intuition = v,
            ("charisma", Value::U8(v)) => self.charisma = v,
            ("edge", Value::U8(v)) => self.edge = v,
            ("magic", Value::OptionU8(v)) => self.magic = v,
            ("resonance", Value::OptionU8(v)) => self.resonance = v,
            ("skills", Value::Skills(v)) => self.skills = v,
            ("knowledge_skills", Value::HashMapStringU8(v)) => self.knowledge_skills = v,
            ("nuyen", Value::U32(v)) => self.nuyen = v,
            ("lifestyle", Value::String(v)) => self.lifestyle = v,
            ("contacts", Value::HashMapStringContact(v)) => self.contacts = v,
            ("qualities", Value::VecQuality(v)) => self.qualities = v,
            ("cyberware", Value::VecString(v)) => self.cyberware = v,
            ("bioware", Value::VecString(v)) => self.bioware = v,
            ("inventory", Value::HashMapStringItem(v)) => {
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
            ("matrix_attributes", Value::OptionMatrixAttributes(v)) => self.matrix_attributes = v,
            _ => {
                return Err(format!(
                    "Invalid attribute-value pair for modification: {} {:#?}",
                    attribute, value
                ))
            }
        }
        Ok(())
    }

    fn add_to_attribute(&mut self, attribute: &str, value: Value) -> Result<(), String> {
        match (attribute, value.clone()) {
            ("nuyen", Value::U32(v)) => self.nuyen = self.nuyen.saturating_add(v),
            ("contacts", Value::HashMapStringContact(v)) => self.contacts.extend(v),
            ("qualities", Value::VecQuality(v)) => self.qualities.extend(v),
            ("cyberware", Value::VecString(v)) => self.cyberware.extend(v),
            ("bioware", Value::VecString(v)) => self.bioware.extend(v),
            ("inventory", Value::HashMapStringItem(v)) => {
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
                ))
            }
        }
        Ok(())
    }

    fn remove_from_attribute(&mut self, attribute: &str, value: Value) -> Result<(), String> {
        match (attribute, value.clone()) {
            ("nuyen", Value::U32(v)) => self.nuyen = self.nuyen.saturating_sub(v),
            ("contacts", Value::HashMapStringContact(v)) => {
                for key in v.keys() {
                    self.contacts.remove(key);
                }
            }
            ("qualities", Value::VecQuality(v)) => self.qualities.retain(|q| !v.contains(q)),
            ("cyberware", Value::VecString(v)) => self.cyberware.retain(|item| !v.contains(item)),
            ("bioware", Value::VecString(v)) => self.bioware.retain(|item| !v.contains(item)),
            ("inventory", Value::HashMapStringItem(v)) => {
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
                ))
            }
        }
        Ok(())
    }
}
