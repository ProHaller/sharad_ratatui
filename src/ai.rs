use crate::character::{
    CharacterSheet, CharacterSheetBuilder, CharacterSheetUpdate, Contact, Item, MatrixAttributes,
    Quality, Race, Skills, UpdateOperation,
};
use crate::dice::{DiceRollRequest, DiceRollResponse, perform_dice_roll};
use crate::error::{AIError, AppError, GameError, ShadowrunError};
use crate::game_state::GameState;
use crate::imager::generate_and_save_image;
use crate::message;
use crate::message::{Message, MessageType};
use async_openai::types::{RequiredAction, RunToolCallObject};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        CreateMessageRequestArgs, CreateRunRequestArgs, CreateThreadRequestArgs, MessageContent,
        MessageRole, Model, RunObject, RunStatus, SubmitToolOutputsRunRequest, ToolsOutputs,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::time::{Duration, Instant};

// TODO: Create a character_sheet_updater routine that verifies the character sheet is updated
// based on in-game events.
// TODO: Create an error channel that will display all error/debug information throughout the app

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConversationState {
    pub assistant_id: String,
    pub thread_id: String,
    pub character_sheet: Option<CharacterSheet>,
}

pub struct GameAI {
    pub client: Client<OpenAIConfig>,
    pub conversation_state: Arc<Mutex<Option<GameConversationState>>>,
    pub debug_callback: Arc<dyn Fn(String) + Send + Sync>,
    pub image_sender: mpsc::UnboundedSender<PathBuf>,
}

impl Clone for GameAI {
    fn clone(&self) -> Self {
        GameAI {
            client: self.client.clone(),
            conversation_state: Arc::clone(&self.conversation_state),
            debug_callback: Arc::clone(&self.debug_callback),
            image_sender: self.image_sender.clone(),
        }
    }
}

impl GameAI {
    pub async fn new(
        api_key: String,
        debug_callback: impl Fn(String) + Send + Sync + 'static,
        image_sender: mpsc::UnboundedSender<PathBuf>,
    ) -> Result<Self, AppError> {
        let openai_config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(openai_config);

        Ok(Self {
            client,
            conversation_state: Arc::new(Mutex::new(None)),
            debug_callback: Arc::new(debug_callback),
            image_sender,
        })
    }

    fn add_debug_message(&self, message: String) {
        (self.debug_callback)(message);
    }

    pub async fn start_new_conversation(
        &self,
        assistant_id: &str,
        initial_game_state: GameConversationState,
    ) -> Result<(), AIError> {
        let thread = self
            .client
            .threads()
            .create(CreateThreadRequestArgs::default().build()?)
            .await?;

        let mut state = self.conversation_state.lock().await;
        *state = Some(GameConversationState {
            assistant_id: assistant_id.to_string(),
            thread_id: thread.id.clone(),
            ..initial_game_state
        });

        let initial_message = CreateMessageRequestArgs::default()
            .role(MessageRole::User)
            .content("Start the game by assisting the player to create a character. Answer in valid json")
            .build()?;

        self.client
            .threads()
            .messages(&thread.id)
            .create(initial_message)
            .await?;

        Ok(())
    }

    pub async fn load_conversation(&mut self, state: GameConversationState) {
        self.add_debug_message("loading conversation state ".to_string());
        let mut conversation_state = self.conversation_state.lock().await;
        *conversation_state = Some(state);
        self.add_debug_message("loaded conversation state".to_string());
    }

    pub async fn send_message(
        &mut self,
        formatted_message: &str,
        game_state: &mut GameState,
    ) -> Result<message::GameMessage, ShadowrunError> {
        let (thread_id, assistant_id) = self
            .get_conversation_ids()
            .await
            .map_err(ShadowrunError::from)?;

        self.add_message_to_thread(&thread_id, formatted_message)
            .await?;

        let run = self.create_run(&thread_id, &assistant_id).await?;

        self.wait_for_run_completion(&thread_id, &run.id, game_state)
            .await?;

        let response = self.get_latest_message(&thread_id).await?;

        self.add_debug_message(format!("\nAI Response: {:#?}\n", response));

        self.update_game_state(game_state, &response)
            .await
            .map_err(ShadowrunError::from)
    }

    async fn wait_for_run_completion(
        &mut self,
        thread_id: &str,
        run_id: &str,
        game_state: &mut GameState,
    ) -> Result<(), ShadowrunError> {
        let timeout_duration = Duration::from_secs(60);
        let start_time = Instant::now();

        loop {
            if start_time.elapsed() > timeout_duration {
                self.cancel_run(thread_id, run_id).await?;
                return Err(AppError::Timeout.into());
            }

            let run = self
                .client
                .threads()
                .runs(thread_id)
                .retrieve(run_id)
                .await?;

            self.add_debug_message(format!("Run status: {:#?}", run.status));
            match run.status {
                RunStatus::Completed => {
                    self.add_debug_message("Run completed".to_string());
                    return Ok(());
                }
                RunStatus::RequiresAction => {
                    self.add_debug_message("Run requires action".to_string());
                    self.handle_required_action(thread_id, run_id, &run, game_state)
                        .await?;
                }
                RunStatus::Failed | RunStatus::Cancelled | RunStatus::Expired => {
                    self.add_debug_message("Run failed, cancelled, or expired".to_string());
                    let _ = self.cancel_run(thread_id, run_id).await;
                    return Err(ShadowrunError::Game(format!(
                        "Run failed with status: {:#?}",
                        run.status
                    )));
                }
                _ => {
                    self.add_debug_message("Run is in progress".to_string());
                    tokio::time::sleep(Duration::from_secs(1)).await
                }
            }
        }
    }

    async fn update_game_state(
        &self,
        game_state: &mut GameState,
        response: &str,
    ) -> Result<message::GameMessage, AppError> {
        self.add_debug_message(format!("Response: {:#?}", response));
        let game_message: message::GameMessage = serde_json::from_str(response).map_err(|e| {
            ShadowrunError::Game(format!(
                "Failed to parse GameMessage: {:#?}\n Message: {:#?}",
                e, response
            ))
        })?;

        if let Some(new_character_sheet) = game_message.character_sheet.clone() {
            self.add_debug_message(format!(
                "Update Game state: Character sheet: {:#?}",
                new_character_sheet
            ));
            self.update_character_sheet(game_state, new_character_sheet)?;
        }

        Ok(game_message)
    }

    pub fn update_character_sheet(
        &self,
        game_state: &mut GameState,
        new_sheet: CharacterSheet,
    ) -> Result<(), ShadowrunError> {
        self.add_debug_message(format!(
            "Update Character sheet: Character sheet: {:#?}",
            new_sheet
        ));
        game_state.main_character_sheet = Some(new_sheet.clone());

        if let Some(existing_character) = game_state
            .characters
            .iter_mut()
            .find(|c| c.name == new_sheet.name)
        {
            *existing_character = new_sheet;
        } else {
            game_state.characters.push(new_sheet);
        }

        Ok(())
    }

    pub async fn get_models(&self) -> Result<Vec<Model>, AppError> {
        // Get models list
        let list_model_response = self
            .client
            .models()
            .list()
            .await
            .map_err(|e| ShadowrunError::OpenAI(e.to_string()))
            .map_err(AppError::Shadowrun)?;

        // Extract Data
        let available_models = list_model_response.data;
        self.add_debug_message(format!("Models available: {:#?}", available_models));
        Ok(available_models)
    }

    pub async fn cancel_run(&self, thread_id: &str, run_id: &str) -> Result<(), AppError> {
        self.client
            .threads()
            .runs(thread_id)
            .cancel(run_id)
            .await
            .map_err(|e| ShadowrunError::OpenAI(e.to_string()))
            .map_err(AppError::Shadowrun)?;
        self.add_debug_message(format!("Run {} cancelled", run_id));
        Ok(())
    }

    async fn handle_required_action(
        &mut self,
        thread_id: &str,
        run_id: &str,
        run: &RunObject,
        game_state: &mut GameState,
    ) -> Result<(), ShadowrunError> {
        self.add_debug_message(format!(
            "Handling required action: {:#?}",
            run.required_action
        ));

        if let Some(required_action) = &run.required_action {
            match required_action.r#type.as_str() {
                "submit_tool_outputs" => {
                    self.handle_tool_outputs(thread_id, run_id, required_action, game_state)
                        .await
                }
                _ => Err(ShadowrunError::Game(format!(
                    "Unknown required action type: {}",
                    required_action.r#type
                ))),
            }
        } else {
            Err(ShadowrunError::Game("No required action found".to_string()))
        }
    }

    async fn handle_tool_outputs(
        &mut self,
        thread_id: &str,
        run_id: &str,
        required_action: &RequiredAction,
        game_state: &mut GameState,
    ) -> Result<(), ShadowrunError> {
        let mut tool_outputs = Vec::new();

        for tool_call in &required_action.submit_tool_outputs.tool_calls {
            self.add_debug_message(format!("Handling tool call: {:#?}", tool_call));

            let output = match tool_call.function.name.as_str() {
                "create_character_sheet" => {
                    self.handle_create_character_sheet(tool_call, game_state)
                        .await?
                }
                "perform_dice_roll" => self.handle_perform_dice_roll(tool_call, game_state)?,
                "generate_character_image" => self.handle_generate_character_image(tool_call)?,
                "update_basic_attributes" => {
                    self.handle_update_basic_attributes(tool_call, game_state)?
                }
                "update_skills" => self.handle_update_skills(tool_call, game_state)?,
                "update_inventory" => self.handle_update_inventory(tool_call, game_state)?,
                "update_qualities" => self.handle_update_qualities(tool_call, game_state)?,
                "update_matrix_attributes" => {
                    self.handle_update_matrix_attributes(tool_call, game_state)?
                }
                "update_contacts" => self.handle_update_contacts(tool_call, game_state)?,
                "update_augmentations" => {
                    self.handle_update_augmentations(tool_call, game_state)?
                }
                _ => {
                    return Err(ShadowrunError::Game(format!(
                        "Unknown function: {}",
                        tool_call.function.name
                    )));
                }
            };

            tool_outputs.push(ToolsOutputs {
                tool_call_id: Some(tool_call.id.clone()),
                output: Some(output),
            });
        }

        self.submit_tool_outputs(thread_id, run_id, tool_outputs)
            .await
    }

    async fn handle_create_character_sheet(
        &mut self,
        tool_call: &RunToolCallObject,
        game_state: &mut GameState,
    ) -> Result<String, ShadowrunError> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;
        let character_sheet = match self.create_character(&args).await {
            Ok(sheet) => sheet,
            Err(e) => {
                self.add_debug_message(format!("Error creating character: {:#?}", e));
                self.create_dummy_character()
            }
        };

        if character_sheet.main {
            game_state.main_character_sheet = Some(character_sheet.clone());
        }
        game_state.characters.push(character_sheet.clone());

        if let Some(state) = &mut *self.conversation_state.lock().await {
            state.character_sheet = Some(character_sheet.clone());
        }
        self.add_debug_message(format!("Character sheet: {:#?}", character_sheet.clone()));

        Ok(serde_json::to_string(&character_sheet)?)
    }

    fn handle_perform_dice_roll(
        &mut self,
        tool_call: &RunToolCallObject,
        game_state: &mut GameState,
    ) -> Result<String, ShadowrunError> {
        let args: DiceRollRequest = serde_json::from_str(&tool_call.function.arguments)?;
        let response = match perform_dice_roll(args, game_state) {
            Ok(response) => {
                self.add_debug_message(format!("Dice roll: {:#?}", response));
                response
            }
            Err(e) => {
                self.add_debug_message(format!("Error performing dice roll: {:#?}", e));
                DiceRollResponse {
                    hits: 0,
                    glitch: false,
                    critical_glitch: false,
                    critical_success: false,
                    dice_results: vec![],
                    success: false,
                }
            }
        };

        Ok(serde_json::to_string(&response)?)
    }

    fn handle_generate_character_image(
        &mut self,
        tool_call: &RunToolCallObject,
    ) -> Result<String, ShadowrunError> {
        let args: Value = serde_json::from_str(&tool_call.function.arguments)?;

        let image_sender = self.image_sender.clone();
        tokio::spawn(async move {
            let path = generate_and_save_image(&args["image_generation_prompt"].to_string())
                .await
                .expect("Something went wrong generating image");
            let _ = image_sender.send(path);
        });

        Ok("Generating image...".to_string())
    }

    fn handle_update_basic_attributes(
        &mut self,
        tool_call: &RunToolCallObject,
        game_state: &mut GameState,
    ) -> Result<String, ShadowrunError> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;
        let character_name = args["character_name"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing character_name".to_string()))?;
        let updates = &args["updates"];

        let character = game_state
            .characters
            .iter_mut()
            .find(|c| c.name == character_name)
            .ok_or_else(|| GameError::CharacterNotFound(character_name.to_string()))?;

        for (attr, value) in updates.as_object().unwrap() {
            let update = CharacterSheetUpdate::UpdateAttribute {
                attribute: attr.to_string(),
                operation: UpdateOperation::Modify(self.parse_value(attr, value)?),
            };
            character.apply_update(update)?;
        }

        if game_state
            .main_character_sheet
            .as_ref()
            .map(|cs| cs.name == character_name)
            .unwrap_or(false)
        {
            game_state.main_character_sheet = Some(character.clone());
        }

        Ok(format!(
            "Updated basic attributes for character: {}",
            character_name
        ))
    }

    fn handle_update_skills(
        &mut self,
        tool_call: &RunToolCallObject,
        game_state: &mut GameState,
    ) -> Result<String, ShadowrunError> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;
        let character_name = args["character_name"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing character_name".to_string()))?;
        let updates = &args["updates"]["skills"];

        let character = game_state
            .characters
            .iter_mut()
            .find(|c| c.name == character_name)
            .ok_or_else(|| GameError::CharacterNotFound(character_name.to_string()))?;

        let update_category = |category: &str,
                               skill_map: &mut HashMap<String, u8>|
         -> Result<(), AppError> {
            if let Some(category_skills) = updates.get(category).and_then(|s| s.as_array()) {
                for skill in category_skills {
                    let name = skill["name"].as_str().ok_or_else(|| {
                        ShadowrunError::Game(format!("Missing skill name in {} category", category))
                    })?;
                    let rating = skill["rating"].as_u64().ok_or_else(|| {
                        ShadowrunError::Game(format!(
                            "Invalid skill rating in {} category",
                            category
                        ))
                    })? as u8;
                    skill_map.insert(name.to_string(), rating);
                }
            }
            Ok(())
        };

        // Update regular skills
        let mut updated_skills = character.skills.clone();
        update_category("combat", &mut updated_skills.combat)?;
        update_category("physical", &mut updated_skills.physical)?;
        update_category("social", &mut updated_skills.social)?;
        update_category("technical", &mut updated_skills.technical)?;

        let skills_update = CharacterSheetUpdate::UpdateAttribute {
            attribute: "skills".to_string(),
            operation: UpdateOperation::Modify(crate::character::Value::Skills(updated_skills)),
        };
        character.apply_update(skills_update)?;

        // Update knowledge skills
        if let Some(knowledge_skills) = updates.get("knowledge") {
            let mut updated_knowledge_skills = character.knowledge_skills.clone();
            if let Some(knowledge_array) = knowledge_skills.as_array() {
                for skill in knowledge_array {
                    let name = skill["name"].as_str().ok_or_else(|| {
                        ShadowrunError::Game("Missing knowledge skill name".to_string())
                    })?;
                    let rating = skill["rating"].as_u64().ok_or_else(|| {
                        ShadowrunError::Game("Invalid knowledge skill rating".to_string())
                    })? as u8;
                    updated_knowledge_skills.insert(name.to_string(), rating);
                }
            }

            let knowledge_update = CharacterSheetUpdate::UpdateAttribute {
                attribute: "knowledge_skills".to_string(),
                operation: UpdateOperation::Modify(crate::character::Value::HashMapStringU8(
                    updated_knowledge_skills,
                )),
            };
            character.apply_update(knowledge_update)?;
        }

        if game_state
            .main_character_sheet
            .as_ref()
            .map(|cs| cs.name == character_name)
            .unwrap_or(false)
        {
            game_state.main_character_sheet = Some(character.clone());
        }

        Ok(format!("Updated skills for character: {}", character_name))
    }

    fn handle_update_inventory(
        &mut self,
        tool_call: &RunToolCallObject,
        game_state: &mut GameState,
    ) -> Result<String, ShadowrunError> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;
        let character_name = args["character_name"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing character_name".to_string()))?;
        let operation = args["operation"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing operation".to_string()))?;
        let items = args.get("items").or_else(|| args.get("item"));

        let character = game_state
            .characters
            .iter_mut()
            .find(|c| c.name == character_name)
            .ok_or_else(|| GameError::CharacterNotFound(character_name.to_string()))?;

        let mut new_items: HashMap<String, Item> = HashMap::new();

        match operation {
            "Remove" => {
                // For removal, we only need the item names
                if let Some(items) = items {
                    let item_names: Vec<String> = if items.is_array() {
                        serde_json::from_value(items.clone())?
                    } else if items.is_object() && items.get("name").is_some() {
                        vec![items["name"].as_str().unwrap().to_string()]
                    } else if items.is_object() && items.get("name").is_none() {
                        items.as_object().unwrap().keys().cloned().collect()
                    } else {
                        return Err(ShadowrunError::Game(
                            "Invalid items format for removal".to_string(),
                        ));
                    };
                    for name in item_names {
                        new_items.insert(
                            name.clone(),
                            Item {
                                name: name.clone(),
                                quantity: 1,
                                description: String::new(),
                            },
                        );
                    }
                }
            }
            "Add" | "Modify" => {
                if let Some(items) = items {
                    if items.is_object() {
                        if items.get("name").is_some() {
                            // Single item
                            let item: Item = serde_json::from_value(items.clone())?;
                            new_items.insert(item.name.clone(), item);
                        } else {
                            // Multiple items
                            for (key, value) in items.as_object().unwrap() {
                                let item = if value.is_object() {
                                    Item {
                                        name: key.clone(),
                                        quantity: value["quantity"].as_u64().unwrap_or(1) as u32,
                                        description: value["description"]
                                            .as_str()
                                            .unwrap_or("")
                                            .to_string(),
                                    }
                                } else {
                                    return Err(ShadowrunError::Game(format!(
                                        "Invalid item format for {}",
                                        key
                                    )));
                                };
                                new_items.insert(key.clone(), item);
                            }
                        }
                    } else {
                        return Err(ShadowrunError::Game(
                            "Invalid items format for add/modify".to_string(),
                        ));
                    }
                } else {
                    return Err(ShadowrunError::Game(
                        "Missing items or item for add/modify".to_string(),
                    ));
                }
            }
            _ => {
                return Err(ShadowrunError::Game(
                    "Invalid inventory operation".to_string(),
                ));
            }
        };

        let update = CharacterSheetUpdate::UpdateAttribute {
            attribute: "inventory".to_string(),
            operation: match operation {
                "Add" => {
                    UpdateOperation::Add(crate::character::Value::HashMapStringItem(new_items))
                }
                "Remove" => {
                    UpdateOperation::Remove(crate::character::Value::HashMapStringItem(new_items))
                }
                "Modify" => {
                    UpdateOperation::Modify(crate::character::Value::HashMapStringItem(new_items))
                }
                _ => unreachable!(),
            },
        };
        character.apply_update(update)?;

        if game_state
            .main_character_sheet
            .as_ref()
            .map(|cs| cs.name == character_name)
            .unwrap_or(false)
        {
            game_state.main_character_sheet = Some(character.clone());
        }

        Ok(format!(
            "Updated inventory for character: {}",
            character_name
        ))
    }

    fn handle_update_qualities(
        &mut self,
        tool_call: &RunToolCallObject,
        game_state: &mut GameState,
    ) -> Result<String, ShadowrunError> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;
        let character_name = args["character_name"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing character_name".to_string()))?;
        let operation = args["operation"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing operation".to_string()))?;
        let qualities = &args["qualities"];

        let character = game_state
            .characters
            .iter_mut()
            .find(|c| c.name == character_name)
            .ok_or_else(|| GameError::CharacterNotFound(character_name.to_string()))?;

        let new_qualities: Vec<Quality> = serde_json::from_value(qualities.clone())?;

        let update = CharacterSheetUpdate::UpdateAttribute {
            attribute: "qualities".to_string(),
            operation: match operation {
                "Add" => UpdateOperation::Add(crate::character::Value::VecQuality(new_qualities)),
                "Remove" => {
                    UpdateOperation::Remove(crate::character::Value::VecQuality(new_qualities))
                }
                _ => {
                    return Err(ShadowrunError::Game(
                        "Invalid qualities operation".to_string(),
                    ));
                }
            },
        };
        character.apply_update(update)?;

        if game_state
            .main_character_sheet
            .as_ref()
            .map(|cs| cs.name == character_name)
            .unwrap_or(false)
        {
            game_state.main_character_sheet = Some(character.clone());
        }
        Ok(format!(
            "Updated qualities for character: {}",
            character_name
        ))
    }

    fn handle_update_matrix_attributes(
        &mut self,
        tool_call: &RunToolCallObject,
        game_state: &mut GameState,
    ) -> Result<String, ShadowrunError> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;
        let character_name = args["character_name"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing character_name".to_string()))?;
        let matrix_attributes = &args["matrix_attributes"];

        let character = game_state
            .characters
            .iter_mut()
            .find(|c| c.name == character_name)
            .ok_or_else(|| GameError::CharacterNotFound(character_name.to_string()))?;

        let new_matrix_attributes: MatrixAttributes =
            serde_json::from_value(matrix_attributes.clone())?;

        let update = CharacterSheetUpdate::UpdateAttribute {
            attribute: "matrix_attributes".to_string(),
            operation: UpdateOperation::Modify(crate::character::Value::OptionMatrixAttributes(
                Some(new_matrix_attributes),
            )),
        };
        character.apply_update(update)?;

        if game_state
            .main_character_sheet
            .as_ref()
            .map(|cs| cs.name == character_name)
            .unwrap_or(false)
        {
            game_state.main_character_sheet = Some(character.clone());
        }
        Ok(format!(
            "Updated matrix attributes for character: {}",
            character_name
        ))
    }

    fn handle_update_contacts(
        &mut self,
        tool_call: &RunToolCallObject,
        game_state: &mut GameState,
    ) -> Result<String, ShadowrunError> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;
        let character_name = args["character_name"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing character_name".to_string()))?;
        let operation = args["operation"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing operation".to_string()))?;
        let contacts = &args["contacts"];

        let character = game_state
            .characters
            .iter_mut()
            .find(|c| c.name == character_name)
            .ok_or_else(|| GameError::CharacterNotFound(character_name.to_string()))?;

        // Deserialize into Vec<Contact>
        let new_contacts_vec: Vec<Contact> = serde_json::from_value(contacts.clone())?;

        // Convert Vec<Contact> into HashMap<String, Contact>
        let new_contacts: HashMap<String, Contact> = new_contacts_vec
            .into_iter()
            .map(|contact| (contact.name.clone(), contact))
            .collect();

        let update = CharacterSheetUpdate::UpdateAttribute {
            attribute: "contacts".to_string(),
            operation: match operation {
                "Add" => UpdateOperation::Add(crate::character::Value::HashMapStringContact(
                    new_contacts,
                )),
                "Remove" => UpdateOperation::Remove(crate::character::Value::HashMapStringContact(
                    new_contacts,
                )),
                "Modify" => UpdateOperation::Modify(crate::character::Value::HashMapStringContact(
                    new_contacts,
                )),
                _ => {
                    return Err(ShadowrunError::Game(
                        "Invalid contacts operation".to_string(),
                    ));
                }
            },
        };
        character.apply_update(update)?;

        if game_state
            .main_character_sheet
            .as_ref()
            .map(|cs| cs.name == character_name)
            .unwrap_or(false)
        {
            game_state.main_character_sheet = Some(character.clone());
        }
        Ok(format!(
            "Updated contacts for character: {}",
            character_name
        ))
    }

    fn handle_update_augmentations(
        &mut self,
        tool_call: &RunToolCallObject,
        game_state: &mut GameState,
    ) -> Result<String, ShadowrunError> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)?;
        let character_name = args["character_name"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing character_name".to_string()))?;
        let operation = args["operation"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing operation".to_string()))?;
        let augmentation_type = args["augmentation_type"]
            .as_str()
            .ok_or_else(|| ShadowrunError::Game("Missing augmentation_type".to_string()))?;
        let augmentations = &args["augmentations"];

        let character = game_state
            .characters
            .iter_mut()
            .find(|c| c.name == character_name)
            .ok_or_else(|| GameError::CharacterNotFound(character_name.to_string()))?;

        let new_augmentations: Vec<String> = serde_json::from_value(augmentations.clone())?;

        let update = CharacterSheetUpdate::UpdateAttribute {
            attribute: augmentation_type.to_string(),
            operation: match operation {
                "Add" => {
                    UpdateOperation::Add(crate::character::Value::VecString(new_augmentations))
                }
                "Remove" => {
                    UpdateOperation::Remove(crate::character::Value::VecString(new_augmentations))
                }
                _ => {
                    return Err(ShadowrunError::Game(
                        "Invalid augmentations operation".to_string(),
                    ));
                }
            },
        };
        character.apply_update(update)?;

        if game_state
            .main_character_sheet
            .as_ref()
            .map(|cs| cs.name == character_name)
            .unwrap_or(false)
        {
            game_state.main_character_sheet = Some(character.clone());
        }

        Ok(format!(
            "{} updated for character '{}'. Operation: {}",
            augmentation_type, character_name, operation
        ))
    }

    // Helper method to parse values based on attribute type
    fn parse_value(
        &self,
        attribute: &str,
        value: &Value,
    ) -> Result<crate::character::Value, ShadowrunError> {
        self.add_debug_message(format!("Parsing value for attribute: {:#?}", attribute));
        match attribute {
            "name" | "gender" | "backstory" | "lifestyle" => Ok(crate::character::Value::String(
                value
                    .as_str()
                    .ok_or_else(|| ShadowrunError::Game("Invalid string value".to_string()))?
                    .to_string(),
            )),
            "race" => Ok(crate::character::Value::Race(
                match value
                    .as_str()
                    .ok_or_else(|| ShadowrunError::Game("Invalid race value".to_string()))?
                {
                    "Human" => Race::Human,
                    "Elf" => Race::Elf,
                    "Dwarf" => Race::Dwarf,
                    "Ork" => Race::Ork,
                    "Troll" => Race::Troll,
                    _ => return Err(ShadowrunError::Game("Invalid race".to_string())),
                },
            )),
            "body" | "agility" | "reaction" | "strength" | "willpower" | "logic" | "intuition"
            | "charisma" | "edge" => Ok(crate::character::Value::U8(
                value
                    .as_u64()
                    .ok_or_else(|| ShadowrunError::Game("Invalid u8 value".to_string()))?
                    as u8,
            )),
            "magic" | "resonance" => Ok(crate::character::Value::OptionU8(
                value.as_u64().map(|v| v as u8),
            )),
            "nuyen" => {
                Ok(crate::character::Value::U32(value.as_u64().ok_or_else(|| {
                    ShadowrunError::Game("Invalid u32 value for nuyen".to_string())
                })? as u32))
            }
            "skills" => Ok(crate::character::Value::Skills(
                serde_json::from_value(value.clone())
                    .map_err(|e| ShadowrunError::Serialization(e.to_string()))?,
            )),
            "knowledge_skills" => Ok(crate::character::Value::HashMapStringU8(
                serde_json::from_value(value.clone())
                    .map_err(|e| ShadowrunError::Serialization(e.to_string()))?,
            )),
            "contacts" => Ok(crate::character::Value::HashMapStringContact(
                serde_json::from_value(value.clone())
                    .map_err(|e| ShadowrunError::Serialization(e.to_string()))?,
            )),
            "qualities" => Ok(crate::character::Value::VecQuality(
                serde_json::from_value(value.clone())
                    .map_err(|e| ShadowrunError::Serialization(e.to_string()))?,
            )),
            "cyberware" | "bioware" => Ok(crate::character::Value::VecString(
                serde_json::from_value(value.clone())
                    .map_err(|e| ShadowrunError::Serialization(e.to_string()))?,
            )),
            "inventory" => Ok(crate::character::Value::HashMapStringItem(
                serde_json::from_value(value.clone())
                    .map_err(|e| ShadowrunError::Serialization(e.to_string()))?,
            )),
            "matrix_attributes" => Ok(crate::character::Value::OptionMatrixAttributes(
                serde_json::from_value(value.clone())
                    .map_err(|e| ShadowrunError::Serialization(e.to_string()))?,
            )),
            _ => Err(ShadowrunError::Game(format!(
                "Unsupported attribute: {}",
                attribute
            ))),
        }
    }

    // Asynchronous method to fetch all messages from a thread, ordered and formatted appropriately.
    pub async fn fetch_all_messages(&self, thread_id: &str) -> Result<Vec<Message>, AIError> {
        let mut all_messages = Vec::new();
        let mut before: Option<String> = None;
        loop {
            let mut params = vec![("order", "desc"), ("limit", "100")];
            if let Some(before_id) = &before {
                params.push(("before", before_id));
            }
            let messages = self
                .client
                .threads()
                .messages(thread_id)
                .list(&params)
                .await?;

            for message in messages.data.into_iter().rev() {
                if let Some(MessageContent::Text(text_content)) = message.content.first() {
                    let message_type = match message.role {
                        MessageRole::User => MessageType::User,
                        MessageRole::Assistant => MessageType::Game,
                    };
                    all_messages.push(Message::new(message_type, text_content.text.value.clone()));
                }
            }

            if messages.has_more {
                before = messages.first_id;
            } else {
                break;
            }
        }
        Ok(all_messages)
    }

    // Asynchronous method to retrieve the latest message from a conversation thread.
    async fn get_latest_message(&self, thread_id: &str) -> Result<String, AIError> {
        self.add_debug_message(format!(
            "Retrieving latest message from thread: {:#?}",
            thread_id
        ));
        let messages = self
            .client
            .threads()
            .messages(thread_id)
            .list(&[("limit", "1")])
            .await?;

        if let Some(latest_message) = messages.data.first() {
            if let Some(MessageContent::Text(text_content)) = latest_message.content.first() {
                self.add_debug_message(format!("Latest message: {:#?}", latest_message,));
                return Ok(text_content.text.value.clone());
            }
        }
        Err(AIError::NoMessageFound)
    }

    // Helper methods
    pub async fn get_conversation_ids(&self) -> Result<(String, String), AppError> {
        self.add_debug_message("Getting conversation IDs".to_string());
        let state = self.conversation_state.lock().await;
        self.add_debug_message(format!("Conversation state: {:#?}", state));
        state
            .as_ref()
            .map(|state| (state.thread_id.clone(), state.assistant_id.clone()))
            .ok_or(AppError::ConversationNotInitialized)
    }

    async fn add_message_to_thread(
        &self,
        thread_id: &str,
        message: &str,
    ) -> Result<(), ShadowrunError> {
        self.add_debug_message(format!(
            "Adding message to thread: {:#?} - {:#?}",
            thread_id, message
        ));
        let message_request = CreateMessageRequestArgs::default()
            .role(MessageRole::User)
            .content(message)
            .build()?;
        self.client
            .threads()
            .messages(thread_id)
            .create(message_request)
            .await?;
        Ok(())
    }

    async fn create_run(
        &self,
        thread_id: &str,
        assistant_id: &str,
    ) -> Result<RunObject, ShadowrunError> {
        self.add_debug_message(format!("Creating run for thread: {:#?}", thread_id));
        let run_request = CreateRunRequestArgs::default()
            .assistant_id(assistant_id)
            .build()?;
        Ok(self
            .client
            .threads()
            .runs(thread_id)
            .create(run_request)
            .await?)
    }

    // Asynchronous method to submit output from a tool during a run.
    async fn submit_tool_outputs(
        &self,
        thread_id: &str,
        run_id: &str,
        tool_outputs: Vec<ToolsOutputs>,
    ) -> Result<(), ShadowrunError> {
        self.add_debug_message(format!("Submitting tool outputs: {:#?}", tool_outputs));
        let submit_request = SubmitToolOutputsRunRequest {
            tool_outputs,
            stream: None,
        };

        self.client
            .threads()
            .runs(thread_id)
            .submit_tool_outputs(run_id, submit_request)
            .await?;

        Ok(())
    }

    // Asynchronous method to create a character based on provided arguments, handling attributes and skills.

    pub async fn create_character(&self, args: &Value) -> Result<CharacterSheet, AIError> {
        self.add_debug_message(format!("Creating character: {:#?}", args));
        // Helper function to extract a string
        fn extract_str(args: &Value, field: &str) -> Result<String, AIError> {
            args.get(field)
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AIError::GameStateParseError(format!("Missing or invalid {}", field))
                })
                .map(String::from)
        }

        // Helper function to extract a u8
        fn extract_u8(args: &Value, field: &str) -> Result<u8, AIError> {
            args.get(field)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    AIError::GameStateParseError(format!("Missing or invalid {}", field))
                })
                .and_then(|v| {
                    u8::try_from(v).map_err(|_| {
                        AIError::GameStateParseError(format!("{} out of range", field))
                    })
                })
        }

        // Helper function to extract an optional u8
        fn extract_u8_opt(args: &Value, field: &str) -> Option<u8> {
            args.get(field)
                .and_then(|v| v.as_u64())
                .and_then(|v| u8::try_from(v).ok())
        }

        // Extract basic information
        let name = extract_str(args, "name")?;
        let race_str = extract_str(args, "race")?;
        let gender = extract_str(args, "gender")?;
        let backstory = extract_str(args, "backstory")?;
        let main = args.get("main").and_then(|v| v.as_bool()).unwrap_or(false);

        // Parse race
        let race = match race_str.as_str() {
            "Human" => Race::Human,
            "Elf" => Race::Elf,
            "Dwarf" => Race::Dwarf,
            "Ork" => Race::Ork,
            "Troll" => Race::Troll,
            _ => return Err(AIError::GameStateParseError("Invalid race".to_string())),
        };

        // Extract attributes
        let attributes = args
            .get("attributes")
            .ok_or_else(|| AIError::GameStateParseError("Missing attributes".to_string()))?;
        let body = extract_u8(attributes, "body")?;
        let agility = extract_u8(attributes, "agility")?;
        let reaction = extract_u8(attributes, "reaction")?;
        let strength = extract_u8(attributes, "strength")?;
        let willpower = extract_u8(attributes, "willpower")?;
        let logic = extract_u8(attributes, "logic")?;
        let intuition = extract_u8(attributes, "intuition")?;
        let charisma = extract_u8(attributes, "charisma")?;
        let edge = extract_u8(attributes, "edge")?;
        let magic = extract_u8_opt(attributes, "magic");
        let resonance = extract_u8_opt(attributes, "resonance");

        // Extract skills
        let skills_obj = args
            .get("skills")
            .ok_or_else(|| AIError::GameStateParseError("Missing skills".to_string()))?;
        let mut skills = Skills {
            combat: HashMap::new(),
            physical: HashMap::new(),
            social: HashMap::new(),
            technical: HashMap::new(),
        };

        for (category, skills_map) in [
            ("combat", &mut skills.combat),
            ("physical", &mut skills.physical),
            ("social", &mut skills.social),
            ("technical", &mut skills.technical),
        ] {
            if let Some(category_array) = skills_obj.get(category).and_then(|v| v.as_array()) {
                for skill in category_array {
                    let name = extract_str(skill, "name")?;
                    let rating = extract_u8(skill, "rating")?;
                    skills_map.insert(name, rating);
                }
            }
        }

        let mut knowledge_skills = HashMap::new();
        if let Some(knowledge_skills_array) = skills_obj.get("knowledge").and_then(|v| v.as_array())
        {
            for skill in knowledge_skills_array {
                let name = extract_str(skill, "name")?;
                let rating = extract_u8(skill, "rating")?;
                knowledge_skills.insert(name, rating);
            }
        }

        // Extract qualities
        let qualities = args
            .get("qualities")
            .and_then(|v| v.as_array())
            .ok_or_else(|| AIError::GameStateParseError("Missing qualities".to_string()))?
            .iter()
            .map(|quality| {
                let name = extract_str(quality, "name")?;
                let positive = quality
                    .get("positive")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| {
                        AIError::GameStateParseError(
                            "Missing or invalid 'positive' in quality".to_string(),
                        )
                    })?;
                Ok(Quality { name, positive })
            })
            .collect::<Result<Vec<Quality>, AIError>>()?;

        // Extract nuyen
        let nuyen = args.get("nuyen").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        // Extract inventory
        let inventory = args
            .get("inventory")
            .and_then(|v| v.get("items"))
            .and_then(|items| items.as_array())
            .map(|items_array| {
                items_array
                    .iter()
                    .filter_map(|item| {
                        let name = item.get("name")?.as_str()?;
                        let quantity = item.get("quantity")?.as_u64()? as u32;
                        let description = item.get("description")?.as_str()?.to_string();

                        Some((
                            name.to_string(),
                            Item {
                                name: name.to_string(),
                                quantity,
                                description,
                            },
                        ))
                    })
                    .collect::<HashMap<String, Item>>()
            })
            .unwrap_or_default();

        // Extract contacts
        let contacts = args
            .get("contacts")
            .and_then(|v| v.as_array())
            .map(|contacts_array| {
                contacts_array
                    .iter()
                    .filter_map(|contact| {
                        let name = contact.get("name")?.as_str()?.to_string();
                        let description = contact.get("description")?.as_str()?.to_string();
                        let loyalty = contact.get("loyalty")?.as_u64()? as u8;
                        let connection = contact.get("connection")?.as_u64()? as u8;

                        Some((
                            name.clone(),
                            Contact {
                                name,
                                description,
                                loyalty,
                                connection,
                            },
                        ))
                    })
                    .collect::<HashMap<String, Contact>>()
            })
            .unwrap_or_default();

        // Create base character sheet using the builder pattern
        let character = CharacterSheetBuilder::new(name, race, gender, backstory, main)
            .body(body)
            .agility(agility)
            .reaction(reaction)
            .strength(strength)
            .willpower(willpower)
            .logic(logic)
            .intuition(intuition)
            .charisma(charisma)
            .edge(edge)
            .magic(magic.unwrap_or(0))
            .resonance(resonance.unwrap_or(0))
            .skills(skills)
            .knowledge_skills(knowledge_skills)
            .qualities(qualities)
            .nuyen(nuyen)
            .inventory(inventory)
            .contacts(contacts)
            .build();

        self.add_debug_message(format!("Character created: {:#?}", character));

        Ok(character)
    }

    // Method to create a dummy character as a fallback during error handling.
    fn create_dummy_character(&self) -> CharacterSheet {
        self.add_debug_message("Creating dummy character.".to_string());
        let dummy_skills = Skills {
            combat: [
                ("Unarmed Combat".to_string(), 1),
                ("Pistols".to_string(), 1),
            ]
            .iter()
            .cloned()
            .collect(),
            physical: [("Running".to_string(), 1), ("Sneaking".to_string(), 1)]
                .iter()
                .cloned()
                .collect(),
            social: [("Etiquette".to_string(), 1), ("Negotiation".to_string(), 1)]
                .iter()
                .cloned()
                .collect(),
            technical: [("Computer".to_string(), 1), ("First Aid".to_string(), 1)]
                .iter()
                .cloned()
                .collect(),
        };
        let dummy_knowledge = HashMap::new();

        CharacterSheetBuilder::new(
            "Dummy Character".to_string(),
            Race::Human,
            "Unspecified".to_string(),
            "This is a dummy character created as a fallback.".to_string(),
            false, // main
        )
        .body(3)
        .agility(3)
        .reaction(3)
        .strength(3)
        .willpower(3)
        .logic(3)
        .intuition(3)
        .charisma(3)
        .edge(3)
        .magic(0)
        .resonance(0)
        .skills(dummy_skills)
        .knowledge_skills(dummy_knowledge)
        .qualities(vec![])
        .nuyen(5000)
        .inventory(HashMap::new())
        .contacts(HashMap::new())
        .build()
    }
}
