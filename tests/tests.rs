// ../tests/tests.rs
use sharad_ratatui::*;
use std::collections::HashMap;
use std::fs;

#[test]
fn test_character_sheet_creation_from_json() {
    // Step 1: Read the dummy JSON file
    let json_str = fs::read_to_string("tests/dummy_create_character_sheet.json")
        .expect("Failed to read dummy create character JSON file");

    // Step 2: Parse the JSON into a serde_json::Value
    let json_value: serde_json::Value =
        serde_json::from_str(&json_str).expect("Failed to parse JSON");

    // Step 3: Extract the function call arguments
    let args = &json_value["function"]["arguments"];

    // Step 4: Create a CharacterSheet from the arguments
    let character_sheet = create_character_from_args(args);

    // Step 5: Verify the created character sheet
    assert_eq!(character_sheet.name, "Alex 'Raven' Hayes");
    assert_eq!(character_sheet.race, Race::Human);
    assert_eq!(character_sheet.gender, "Male");
    assert_eq!(character_sheet.attributes.body, 4);
    assert_eq!(character_sheet.attributes.agility, 5);
    assert_eq!(character_sheet.attributes.charisma, 3);

    // Check skills
    assert_eq!(character_sheet.skills.combat.get("Pistols"), Some(&4));
    assert_eq!(character_sheet.skills.physical.get("Stealth"), Some(&6));

    // Check qualities
    assert!(character_sheet.qualities.contains(&Quality {
        name: "Natural Athlete".to_string(),
        positive: true
    }));

    // Check contacts
    assert!(
        character_sheet
            .contacts
            .contains_key("Jenna 'Fixer' Morgan")
    );

    println!("Created Character Sheet: {:?}", character_sheet);
}

fn create_character_from_args(args: &serde_json::Value) -> CharacterSheet {
    let name = args["name"]
        .as_str()
        .expect("Expected a valid String")
        .to_string();
    let race = match args["race"].as_str().expect("Expected a valid String") {
        "Human" => Race::Human,
        "Elf" => Race::Elf,
        "Troll" => Race::Troll,
        "Dwarf" => Race::Dwarf,
        "Ork" => Race::Ork,
        _ => panic!("Unknown race"),
    };
    let gender = args["gender"]
        .as_str()
        .expect("Expected a valid String")
        .to_string();
    let backstory = args["backstory"]
        .as_str()
        .expect("Expected a valid String")
        .to_string();
    let main = args["main"].as_bool().expect("Expected an argument main");

    let mut builder = CharacterSheetBuilder::new(name, race, gender, backstory, main);

    // Set attributes
    let attributes = &args["attributes"];
    builder = builder
        .body(attributes["body"].as_u64().expect("Expected some u64") as u8)
        .agility(attributes["agility"].as_u64().expect("Expected some u64") as u8)
        .reaction(attributes["reaction"].as_u64().expect("Expected some u64") as u8)
        .strength(attributes["strength"].as_u64().expect("Expected some u64") as u8)
        .willpower(attributes["willpower"].as_u64().expect("Expected some u64") as u8)
        .logic(attributes["logic"].as_u64().expect("Expected some u64") as u8)
        .intuition(attributes["intuition"].as_u64().expect("Expected some u64") as u8)
        .charisma(attributes["charisma"].as_u64().expect("Expected some u64") as u8)
        .edge(attributes["edge"].as_u64().expect("Expected some u64") as u8)
        .magic(attributes["magic"].as_u64().expect("Expected some u64") as u8)
        .resonance(attributes["resonance"].as_u64().expect("Expected some u64") as u8);

    // Set skills
    let mut skills = Skills {
        combat: HashMap::new(),
        physical: HashMap::new(),
        social: HashMap::new(),
        technical: HashMap::new(),
    };
    let skill_categories = ["combat", "physical", "social", "technical"];
    for category in &skill_categories {
        for skill in args["skills"][category]
            .as_array()
            .expect("Expected some array")
        {
            let skill_name = skill["name"]
                .as_str()
                .expect("Expected some string")
                .to_string();
            let skill_rating = skill["rating"].as_u64().expect("Expected some u64") as u8;
            match *category {
                "combat" => skills.combat.insert(skill_name, skill_rating),
                "physical" => skills.physical.insert(skill_name, skill_rating),
                "social" => skills.social.insert(skill_name, skill_rating),
                "technical" => skills.technical.insert(skill_name, skill_rating),
                _ => panic!("Unknown skill category"),
            };
        }
    }
    builder = builder.skills(skills);

    // Set knowledge skills
    let mut knowledge_skills = HashMap::new();
    for skill in args["skills"]["knowledge"]
        .as_array()
        .expect("Expected some array")
    {
        let skill_name = skill["name"]
            .as_str()
            .expect("Expected some String")
            .to_string();
        let skill_rating = skill["rating"].as_u64().expect("Expected some u64") as u8;
        knowledge_skills.insert(skill_name, skill_rating);
    }
    builder = builder.knowledge_skills(knowledge_skills);

    // Set qualities
    let qualities = args["qualities"]
        .as_array()
        .expect("Expected some array")
        .iter()
        .map(|q| Quality {
            name: q["name"]
                .as_str()
                .expect("Expected some String")
                .to_string(),
            positive: q["positive"].as_bool().expect("Expected some bool"),
        })
        .collect();
    builder = builder.qualities(qualities);

    // Set nuyen
    builder = builder.nuyen(args["nuyen"].as_u64().expect("Expected some u64") as u32);

    // Set contacts
    let mut contacts = HashMap::new();
    for contact in args["contacts"].as_array().expect("Expected some array") {
        let name = contact["name"]
            .as_str()
            .expect("Expected some String")
            .to_string();
        contacts.insert(
            name.clone(),
            Contact {
                name,
                description: contact["description"]
                    .as_str()
                    .expect("Expected some String")
                    .to_string(),
                loyalty: contact["loyalty"].as_u64().expect("Expected some u64") as u8,
                connection: contact["connection"].as_u64().expect("Expected some u64") as u8,
            },
        );
    }
    builder = builder.contacts(contacts);

    builder.build()
}

#[test]
fn test_character_sheet_update_from_json() {
    // Step 1: Read the dummy JSON file
    let json_str = fs::read_to_string("tests/dummy_create_character_sheet.json")
        .expect("Failed to read dummy create character JSON file");

    // Step 2: Parse the JSON into a serde_json::Value
    let json_value: serde_json::Value =
        serde_json::from_str(&json_str).expect("Failed to parse JSON");

    // Step 3: Extract the function call arguments
    let args = &json_value["function"]["arguments"];

    // Step 4: Create a CharacterSheet from the arguments
    let mut character_sheet = create_character_from_args(args);
    // Step 1: Read the dummy JSON file
    let json_str = fs::read_to_string("tests/dummy_update_character_sheet.json")
        .expect("Expected to read dummy update character JSON file");

    // Step 2: Parse the JSON into a serde_json::Value
    let json_value: serde_json::Value =
        serde_json::from_str(&json_str).expect("Failed to parse JSON");

    // Step 3: Extract the function call arguments
    let args = &json_value["function"]["arguments"];

    // Step 5: Parse the update arguments
    let update = CharacterSheetUpdate::UpdateAttribute {
        attribute: args["update"]["attribute"]
            .as_str()
            .expect("Expected some String")
            .to_string(),
        operation: match args["update"]["operation"]
            .as_str()
            .expect("Expected some String")
        {
            "Add" => UpdateOperation::Add(Value::VecQuality(
                args["update"]["value"]
                    .as_array()
                    .expect("Expected some array")
                    .iter()
                    .map(|quality| Quality {
                        name: quality["name"]
                            .as_str()
                            .expect("Expected some String")
                            .to_string(),
                        positive: quality["positive"].as_bool().expect("Expected some bool"),
                    })
                    .collect(),
            )),
            _ => panic!("Unsupported operation in test"),
        },
    };

    println!("{:#?}", update);
    // Step 6: Apply the update
    character_sheet
        .apply_update(update)
        .expect("Failed to apply update");

    println!("{:#?}", character_sheet);
    // Step 7: Verify the changes
    assert!(
        character_sheet
            .qualities
            .iter()
            .any(|quality| quality.name == "Street Samurai" && quality.positive)
    );

    println!("Updated Character Sheet: {:?}", character_sheet);
}
