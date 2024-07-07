use crate::app::App;
use crate::character::{CharacterSheet, Quality, Race, Skills};
use crate::message::{GameMessage, Message, MessageType};
use async_openai::{
    config::OpenAIConfig,
    types::{
        AssistantTools, AssistantToolsFunction, CreateAssistantRequestArgs,
        CreateMessageRequestArgs, CreateRunRequestArgs, CreateThreadRequestArgs, FunctionObject,
        MessageContent, MessageRole, RequiredAction, RunObject, RunStatus,
        SubmitToolOutputsRunRequest, ToolsOutputs,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{timeout, Duration, Instant};

#[derive(Debug, Serialize, Deserialize)]
pub struct GameConversationState {
    pub assistant_id: String,
    pub thread_id: String,
    pub character_sheet: Option<CharacterSheet>,
}

#[derive(Debug, thiserror::Error)]
pub enum AIError {
    #[error("OpenAI API error: {0}")]
    OpenAI(#[from] async_openai::error::OpenAIError),
    #[error("Conversation not initialized")]
    ConversationNotInitialized,
    #[error("Timeout occurred")]
    Timeout,
    #[error("No message found")]
    NoMessageFound,
    #[error("Failed to parse game state: {0}")]
    GameStateParseError(String),
}

impl From<serde_json::Error> for AIError {
    fn from(err: serde_json::Error) -> AIError {
        AIError::GameStateParseError(err.to_string())
    }
}

pub struct GameAI {
    client: Client<OpenAIConfig>,
    pub conversation_state: Option<GameConversationState>,
    debug_callback: Arc<dyn Fn(String) + Send + Sync>,
    app_ref: Option<Arc<Mutex<App>>>, // Add this line
}

impl GameAI {
    pub fn new(
        api_key: String,
        debug_callback: impl Fn(String) + Send + Sync + 'static,
        app_ref: Arc<Mutex<App>>, // Add this line
    ) -> Result<Self, AIError> {
        let openai_config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(openai_config);

        Ok(Self {
            client,
            conversation_state: None,
            debug_callback: Arc::new(debug_callback),
            app_ref: Some(app_ref), // Add this line
        })
    }

    // Add a method to set the app reference after initialization
    pub fn set_app_ref(&mut self, app_ref: Arc<Mutex<App>>) {
        self.app_ref = Some(app_ref);
    }

    fn add_debug_message(&self, message: String) {
        (self.debug_callback)(message);
    }

    #[allow(dead_code)]
    pub async fn create_game_assistant(
        &self,
        name: &str,
        instructions: &str,
    ) -> Result<String, AIError> {
        let assistant = self
            .client
            .assistants()
            .create(
                CreateAssistantRequestArgs::default()
                    .name(name)
                    .instructions(instructions)
                    .model("gpt-4o")
                    .tools(vec![AssistantTools::CodeInterpreter])
                    .build()?,
            )
            .await?;

        Ok(assistant.id)
    }

    pub async fn start_new_conversation(
        &mut self,
        assistant_id: &str,
        initial_game_state: GameConversationState,
    ) -> Result<(), AIError> {
        let thread = self
            .client
            .threads()
            .create(CreateThreadRequestArgs::default().build()?)
            .await?;

        self.conversation_state = Some(GameConversationState {
            assistant_id: assistant_id.to_string(),
            thread_id: thread.id.clone(),
            ..initial_game_state
        });

        let initial_message = CreateMessageRequestArgs::default()
            .role(MessageRole::User)
            .content("Start a new game. Answer in valid json")
            .build()?;

        self.client
            .threads()
            .messages(&thread.id)
            .create(initial_message)
            .await?;

        Ok(())
    }

    pub async fn load_conversation(&mut self, state: GameConversationState) {
        self.conversation_state = Some(state);
    }

    pub async fn send_message(&mut self, message: &str) -> Result<GameMessage, AIError> {
        // Extract necessary information from conversation_state
        let (thread_id, assistant_id) = match &self.conversation_state {
            Some(state) => (state.thread_id.clone(), state.assistant_id.clone()),
            None => return Err(AIError::ConversationNotInitialized),
        };

        // Check if there's an active run
        let active_runs = self
            .client
            .threads()
            .runs(&thread_id)
            .list(&[("limit", "1")])
            .await?;

        if let Some(run) = active_runs.data.first() {
            if run.status == RunStatus::InProgress || run.status == RunStatus::Queued {
                // Wait for the active run to complete
                self.wait_for_run_completion(&thread_id, &run.id).await?;
            }
        }

        // Now that we're sure no run is active, we can add a new message
        let message_request = CreateMessageRequestArgs::default()
            .role(MessageRole::User)
            .content(message)
            .build()?;

        self.client
            .threads()
            .messages(&thread_id)
            .create(message_request)
            .await?;

        let run_request = CreateRunRequestArgs::default()
            .assistant_id(&assistant_id)
            .build()?;

        let run = self
            .client
            .threads()
            .runs(&thread_id)
            .create(run_request)
            .await?;

        // Wait for the new run to complete
        self.wait_for_run_completion(&thread_id, &run.id).await?;

        let response = self.get_latest_message(&thread_id).await?;
        self.update_game_state(&response)?;

        let game_message: GameMessage = serde_json::from_str(&response).map_err(|e| {
            AIError::GameStateParseError(format!("Failed to parse GameMessage: {}", e))
        })?;

        Ok(game_message)
    }

    async fn wait_for_run_completion(
        &mut self,
        thread_id: &str,
        run_id: &str,
    ) -> Result<(), AIError> {
        let timeout_duration = Duration::from_secs(300); // 5 minutes timeout
        let start_time = Instant::now();
        let mut requires_action_attempts = 0;
        const MAX_REQUIRES_ACTION_ATTEMPTS: u32 = 5;

        loop {
            if start_time.elapsed() > timeout_duration {
                self.add_debug_message("Run timed out".to_string());
                return Err(AIError::Timeout);
            }

            let run = self
                .client
                .threads()
                .runs(thread_id)
                .retrieve(run_id)
                .await?;

            self.add_debug_message(format!(
                "Run status, wait for run completion: {:?}",
                run.status
            ));

            match run.status {
                RunStatus::Completed => {
                    self.add_debug_message("Run completed successfully".to_string());
                    return Ok(());
                }
                RunStatus::RequiresAction => {
                    self.add_debug_message(
                        "Run requires action, handling function call".to_string(),
                    );
                    match self.handle_required_action(thread_id, run_id, &run).await {
                        Ok(_) => {
                            requires_action_attempts = 0;
                        }
                        Err(e) => {
                            requires_action_attempts += 1;
                            if requires_action_attempts >= MAX_REQUIRES_ACTION_ATTEMPTS {
                                self.add_debug_message(
                                    "Max attempts reached, creating dummy character".to_string(),
                                );
                                let dummy_character = self.create_dummy_character();
                                self.submit_tool_output(
                                    thread_id,
                                    run_id,
                                    "create_character_sheet",
                                    &dummy_character,
                                )
                                .await?;
                            } else {
                                self.add_debug_message(format!(
                                    "Error handling required action: {:?}",
                                    e
                                ));
                            }
                        }
                    }
                }
                RunStatus::Failed | RunStatus::Cancelled | RunStatus::Expired => {
                    let error_message = format!("Run failed with status: {:?}", run.status);
                    self.add_debug_message(error_message.clone());
                    return Err(AIError::GameStateParseError(error_message));
                }
                _ => {
                    self.add_debug_message("Run still in progress, waiting...".to_string());
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    async fn handle_required_action(
        &mut self,
        thread_id: &str,
        run_id: &str,
        run: &RunObject,
    ) -> Result<(), AIError> {
        if let Some(required_action) = &run.required_action {
            if required_action.r#type == "submit_tool_outputs" {
                for tool_call in &required_action.submit_tool_outputs.tool_calls {
                    self.add_debug_message(format!("Processing tool call: {:?}", tool_call));
                    match tool_call.function.name.as_str() {
                        "create_character_sheet" => {
                            let args: serde_json::Value =
                                serde_json::from_str(&tool_call.function.arguments)?;
                            let character_sheet = match self.create_character(&args).await {
                                Ok(sheet) => sheet,
                                Err(e) => {
                                    self.add_debug_message(format!(
                                        "Error creating character: {:?}",
                                        e
                                    ));
                                    self.create_dummy_character()
                                }
                            };

                            // Update the conversation state with the new character sheet
                            if let Some(state) = &mut self.conversation_state {
                                state.character_sheet = Some(character_sheet.clone());
                                self.add_debug_message(
                                    "Character sheet updated in conversation state".to_string(),
                                );
                            }

                            self.submit_tool_output(
                                thread_id,
                                run_id,
                                &tool_call.id,
                                &character_sheet,
                            )
                            .await?;
                        }
                        _ => {
                            return Err(AIError::GameStateParseError(format!(
                                "Unknown function: {}",
                                tool_call.function.name
                            )))
                        }
                    }
                }
                Ok(())
            } else {
                Err(AIError::GameStateParseError(format!(
                    "Unknown required action type: {}",
                    required_action.r#type
                )))
            }
        } else {
            Err(AIError::GameStateParseError(
                "No required action found".to_string(),
            ))
        }
    }

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
                        _ => MessageType::System,
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

    async fn get_latest_message(&self, thread_id: &str) -> Result<String, AIError> {
        let messages = self
            .client
            .threads()
            .messages(thread_id)
            .list(&[("limit", "1")])
            .await?;

        if let Some(latest_message) = messages.data.first() {
            if let Some(MessageContent::Text(text_content)) = latest_message.content.first() {
                return Ok(text_content.text.value.clone());
            }
        }
        Err(AIError::NoMessageFound)
    }

    fn update_game_state(&mut self, response: &str) -> Result<(), AIError> {
        self.add_debug_message(format!("Updating game state with response: {}", response));

        // Parse the response as a GameMessage
        let mut game_message: GameMessage = serde_json::from_str(response).map_err(|e| {
            AIError::GameStateParseError(format!("Failed to parse GameMessage: {}", e))
        })?;

        if let Some(state) = &self.conversation_state {
            // Include the character sheet from the conversation state
            game_message.character_sheet = state.character_sheet.clone();
            self.add_debug_message("Character sheet included from conversation state".to_string());
        } else {
            self.add_debug_message("No active conversation state".to_string());
        }

        Ok(())
    }

    async fn submit_tool_output(
        &self,
        thread_id: &str,
        run_id: &str,
        tool_call_id: &str,
        output: &impl Serialize,
    ) -> Result<(), AIError> {
        let tool_output = serde_json::to_string(output)
            .map_err(|e| AIError::GameStateParseError(e.to_string()))?;

        let submit_request = SubmitToolOutputsRunRequest {
            tool_outputs: vec![ToolsOutputs {
                tool_call_id: Some(tool_call_id.to_string()),
                output: Some(tool_output),
            }],
            stream: None,
        };

        self.client
            .threads()
            .runs(thread_id)
            .submit_tool_outputs(run_id, submit_request)
            .await?;

        Ok(())
    }

    async fn create_character(&self, args: &Value) -> Result<CharacterSheet, AIError> {
        self.add_debug_message(format!("Creating character from args: {:?}", args));

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

        // Extract basic information
        let name = extract_str(args, "name")?;
        let race_str = extract_str(args, "race")?;
        let gender = extract_str(args, "gender")?;
        let backstory = extract_str(args, "backstory")?;

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
            .and_then(|v| v.as_object())
            .ok_or_else(|| AIError::GameStateParseError("Missing attributes".to_string()))?;

        let body = extract_u8(&Value::Object(attributes.clone()), "body")?;
        let agility = extract_u8(&Value::Object(attributes.clone()), "agility")?;
        let reaction = extract_u8(&Value::Object(attributes.clone()), "reaction")?;
        let strength = extract_u8(&Value::Object(attributes.clone()), "strength")?;
        let willpower = extract_u8(&Value::Object(attributes.clone()), "willpower")?;
        let logic = extract_u8(&Value::Object(attributes.clone()), "logic")?;
        let intuition = extract_u8(&Value::Object(attributes.clone()), "intuition")?;
        let charisma = extract_u8(&Value::Object(attributes.clone()), "charisma")?;
        let edge = extract_u8(&Value::Object(attributes.clone()), "edge")?;
        let magic = extract_u8(&Value::Object(attributes.clone()), "magic")?;
        let resonance = extract_u8(&Value::Object(attributes.clone()), "resonance")?;

        // Extract skills
        let skills_obj = args
            .get("skills")
            .and_then(|v| v.as_object())
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
            let category_array = skills_obj
                .get(category)
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    AIError::GameStateParseError(format!("Missing {} skills array", category))
                })?;

            for skill in category_array {
                let name = extract_str(skill, "name")?;
                let rating = extract_u8(skill, "rating")?;
                skills_map.insert(name, rating);
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

        // Create base character sheet
        let mut character = CharacterSheet::new(
            name, race, gender, backstory, body, agility, reaction, strength, willpower, logic,
            intuition, charisma, edge, magic, resonance, skills, qualities,
        );

        // Apply race modifiers and update derived attributes
        character.apply_race_modifiers(character.race.clone());
        character.update_derived_attributes();

        self.add_debug_message("Character creation successful".to_string());

        Ok(character)
    }

    fn create_dummy_character(&self) -> CharacterSheet {
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

        CharacterSheet::new(
            "Dummy Character".to_string(),
            Race::Human,
            "Unspecified".to_string(),
            "This is a dummy character created as a fallback.".to_string(),
            3, // body
            3, // agility
            3, // reaction
            3, // strength
            3, // willpower
            3, // logic
            3, // intuition
            3, // charisma
            3, // edge
            0, // magic
            0, // resonance
            dummy_skills,
            vec![], // qualities
        )
    }
}
