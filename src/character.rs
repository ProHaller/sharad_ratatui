use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Race {
    Human,
    Elf,
    Dwarf,
    Ork,
    Troll,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSheet {
    // Personal Information
    pub name: String,
    pub race: Race,
    pub gender: String,
    pub backstory: String,

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
    pub initiative: (u8, u8), // (Base, Dice)
    pub essence: f32,
    pub edge_points: u8,
    pub physical_monitor: u8,
    pub stun_monitor: u8,

    // Derived Attributes
    pub armor: u8,
    pub physical_limit: u8,
    pub mental_limit: u8,
    pub social_limit: u8,

    // Skills
    pub skills: Skills,
    pub knowledge_skills: HashMap<String, u8>,

    // Other Attributes
    pub nuyen: u32,
    pub lifestyle: String,
    pub contacts: Vec<Contact>,
    pub qualities: Vec<Quality>,
    pub cyberware: Vec<String>,
    pub bioware: Vec<String>,
    pub inventory: Vec<Item>,
    pub matrix_attributes: Option<MatrixAttributes>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skills {
    pub combat: HashMap<String, u8>,
    pub physical: HashMap<String, u8>,
    pub social: HashMap<String, u8>,
    pub technical: HashMap<String, u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    pub loyalty: u8,
    pub connection: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quality {
    pub name: String,
    pub positive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub quantity: u32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixAttributes {
    pub attack: u8,
    pub sleaze: u8,
    pub data_processing: u8,
    pub firewall: u8,
}

impl CharacterSheet {
    pub fn new(
        name: String,
        race: Race,
        gender: String,
        backstory: String,
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
    ) -> Self {
        let mut sheet = CharacterSheet {
            name,
            race: race.clone(),
            gender,
            backstory,
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
            nuyen: 0,
            lifestyle: "Street".to_string(),
            contacts: Vec::new(),
            qualities,
            cyberware: Vec::new(),
            bioware: Vec::new(),
            inventory: Vec::new(),
            matrix_attributes: None,
        };

        sheet.apply_race_modifiers(sheet.race.clone());
        sheet.update_derived_attributes();
        sheet
    }

    pub fn roll_attribute() -> u8 {
        let mut rng = rand::thread_rng();
        let mut total = 0;
        for _ in 0..6 {
            let mut roll = rng.gen_range(1..=6);
            while roll == 6 {
                total += 1;
                roll = rng.gen_range(1..=6);
            }
            total += roll;
        }
        total.min(6) // Cap at 6 initially, race modifiers will be applied later
    }

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

    // Helper method to get all active skills
    pub fn get_all_active_skills(&self) -> HashMap<String, u8> {
        let mut all_skills = HashMap::new();
        all_skills.extend(self.skills.combat.clone());
        all_skills.extend(self.skills.physical.clone());
        all_skills.extend(self.skills.social.clone());
        all_skills.extend(self.skills.technical.clone());
        all_skills
    }
}
