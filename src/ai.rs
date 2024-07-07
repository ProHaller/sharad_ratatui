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
use serde_json::{json, Map};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{timeout, Duration, Instant};

#[derive(Debug, Serialize, Deserialize)]
pub struct GameConversationState {
    pub assistant_id: String,
    pub thread_id: String,
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
}

impl GameAI {
    pub fn new(
        api_key: String,
        debug_callback: impl Fn(String) + Send + Sync + 'static,
    ) -> Result<Self, AIError> {
        let openai_config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(openai_config);

        Ok(Self {
            client,
            conversation_state: None,
            debug_callback: Arc::new(debug_callback),
        })
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

    pub async fn send_message(&mut self, message: &str) -> Result<String, AIError> {
        let state = self
            .conversation_state
            .as_ref()
            .ok_or(AIError::ConversationNotInitialized)?;

        // Check if there's an active run
        let active_runs = self
            .client
            .threads()
            .runs(&state.thread_id)
            .list(&[("limit", "1")])
            .await?;

        if let Some(run) = active_runs.data.first() {
            if run.status == RunStatus::InProgress || run.status == RunStatus::Queued {
                // Wait for the active run to complete
                self.wait_for_run_completion(&state.thread_id, &run.id)
                    .await?;
            }
        }

        // Now that we're sure no run is active, we can add a new message
        let message_request = CreateMessageRequestArgs::default()
            .role(MessageRole::User)
            .content(message)
            .build()?;

        self.client
            .threads()
            .messages(&state.thread_id)
            .create(message_request)
            .await?;

        let run_request = CreateRunRequestArgs::default()
            .assistant_id(&state.assistant_id)
            .build()?;

        let run = self
            .client
            .threads()
            .runs(&state.thread_id)
            .create(run_request)
            .await?;

        // Wait for the new run to complete
        self.wait_for_run_completion(&state.thread_id, &run.id)
            .await?;

        let response = self.get_latest_message(&state.thread_id).await?;
        self.update_game_state(&response)?;

        Ok(response)
    }

    async fn wait_for_run_completion(&self, thread_id: &str, run_id: &str) -> Result<(), AIError> {
        let timeout_duration = Duration::from_secs(300); // 5 minutes timeout
        let start_time = Instant::now();
        let mut requires_action_attempts = 0;
        const MAX_REQUIRES_ACTION_ATTEMPTS: u32 = 3;

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
                    let response = self.get_latest_message(thread_id).await?;
                    self.add_debug_message(format!("Completed run Status: {}", response));
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
        &self,
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
                            self.add_debug_message(format!(
                                "Character creation arguments: {:?}",
                                args
                            ));
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
                self.add_debug_message(format!("latest_message: {:?}", text_content.clone()));
                return Ok(text_content.text.value.clone());
            }
        }
        Err(AIError::NoMessageFound)
    }

    fn update_game_state(&mut self, response: &str) -> Result<(), AIError> {
        // This is a placeholder. You'll need to implement proper parsing based on your AI's response format.
        if let Some(_state) = &mut self.conversation_state {
            // Example parsing, adjust according to your actual AI response format
        }
        Ok(())
    }

    pub fn get_game_state(&self) -> Option<&GameConversationState> {
        self.conversation_state.as_ref()
    }

    pub fn update_api_key(&mut self, new_api_key: String) {
        let new_config = OpenAIConfig::new().with_api_key(new_api_key);
        self.client = Client::with_config(new_config);
    }

    pub async fn create_character_with_ai(&self) -> Result<CharacterSheet, AIError> {
        self.add_debug_message("Starting character creation with AI".to_string());
        let state = self
            .conversation_state
            .as_ref()
            .ok_or(AIError::ConversationNotInitialized)?;

        self.add_debug_message("Creating run for character creation".to_string());
        let create_character_sheet = AssistantTools::Function(AssistantToolsFunction {
            function: FunctionObject {
                name: "create_character_sheet".to_string(),
                description: Some("Create a character sheet for a Shadowrun character".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "race": { "type": "string", "enum": ["Human", "Elf", "Dwarf", "Ork", "Troll"] },
                        "gender": { "type": "string" },
                        "backstory": { "type": "string" },
                        "attributes": {
                            "type": "object",
                            "properties": {
                                "body": { "type": "integer", "minimum": 1, "maximum": 6 },
                                "agility": { "type": "integer", "minimum": 1, "maximum": 6 },
                                "reaction": { "type": "integer", "minimum": 1, "maximum": 6 },
                                "strength": { "type": "integer", "minimum": 1, "maximum": 6 },
                                "willpower": { "type": "integer", "minimum": 1, "maximum": 6 },
                                "logic": { "type": "integer", "minimum": 1, "maximum": 6 },
                                "intuition": { "type": "integer", "minimum": 1, "maximum": 6 },
                                "charisma": { "type": "integer", "minimum": 1, "maximum": 6 },
                                "edge": { "type": "integer", "minimum": 1, "maximum": 6 },
                                "magic": { "type": "integer", "minimum": 0, "maximum": 6 },
                                "resonance": { "type": "integer", "minimum": 0, "maximum": 6 }
                            },
                            "required": ["body", "agility", "reaction", "strength", "willpower", "logic", "intuition", "charisma", "edge"]
                        },
                        "skills": {
                            "type": "object",
                            "additionalProperties": { "type": "integer", "minimum": 0, "maximum": 6 }
                        },
                        "qualities": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string" },
                                    "positive": { "type": "boolean" }
                                },
                                "required": ["name", "positive"]
                            }
                        }
                    },
                    "required": ["name", "race", "gender", "backstory", "attributes", "skills"]
                })),
            },
        });

        let request = CreateRunRequestArgs::default()
            .assistant_id(&state.assistant_id)
            .tools(vec![create_character_sheet])
            .build()?;

        let mut run = self
            .client
            .threads()
            .runs(&state.thread_id)
            .create(request)
            .await?;

        // Wait for the run to complete with a timeout
        let timeout_duration = Duration::from_secs(30);
        loop {
            match timeout(
                timeout_duration,
                self.client
                    .threads()
                    .runs(&state.thread_id)
                    .retrieve(&run.id),
            )
            .await
            {
                Ok(Ok(updated_run)) => {
                    run = updated_run;
                    if run.status == RunStatus::Completed {
                        break;
                    }
                }
                Ok(Err(e)) => return Err(AIError::OpenAI(e)),
                Err(_) => return Err(AIError::Timeout),
            }
        }

        // Retrieve the messages after the run is completed
        let messages = self
            .client
            .threads()
            .messages(&state.thread_id)
            .list(&[("limit", "1"), ("order", "desc")])
            .await?;

        if let Some(message) = messages.data.first() {
            if let Some(content) = message.content.first() {
                if let MessageContent::Text(text) = content {
                    // Try to parse the content as a CharacterSheet
                    match serde_json::from_str::<CharacterSheet>(&text.text.value) {
                        Ok(character_sheet) => Ok(character_sheet),
                        Err(e) => Err(AIError::GameStateParseError(format!(
                            "Failed to parse character sheet: {}. Raw JSON: {}",
                            e, text.text.value
                        ))),
                    }
                } else {
                    Err(AIError::GameStateParseError(
                        "Unexpected message content type".to_string(),
                    ))
                }
            } else {
                Err(AIError::GameStateParseError(
                    "No content in message".to_string(),
                ))
            }
        } else {
            Err(AIError::GameStateParseError(
                "No valid response found for character creation".to_string(),
            ))
        }
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

    async fn create_character(&self, args: &serde_json::Value) -> Result<CharacterSheet, AIError> {
        self.add_debug_message(format!("Creating character from args: {:?}", args));

        // Helper function to extract a string
        fn extract_str(args: &serde_json::Value, field: &str) -> Result<String, AIError> {
            args.get(field)
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AIError::GameStateParseError(format!("Missing or invalid {}", field))
                })
                .map(String::from)
        }

        // Helper function to extract a u8
        fn extract_u8(args: &serde_json::Value, field: &str) -> Result<u8, AIError> {
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
        let attributes_obj = args
            .get("attributes")
            .ok_or_else(|| AIError::GameStateParseError("Missing attributes".to_string()))?;

        let body = extract_u8(attributes_obj, "body")?;
        let agility = extract_u8(attributes_obj, "agility")?;
        let reaction = extract_u8(attributes_obj, "reaction")?;
        let strength = extract_u8(attributes_obj, "strength")?;
        let willpower = extract_u8(attributes_obj, "willpower")?;
        let logic = extract_u8(attributes_obj, "logic")?;
        let intuition = extract_u8(attributes_obj, "intuition")?;
        let charisma = extract_u8(attributes_obj, "charisma")?;
        let edge = extract_u8(attributes_obj, "edge")?;
        let magic = attributes_obj
            .get("magic")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8)
            .unwrap_or(0);
        let resonance = attributes_obj
            .get("resonance")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8)
            .unwrap_or(0);

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
            let category_obj = skills_obj
                .get(category)
                .and_then(|v| v.as_object())
                .ok_or_else(|| {
                    AIError::GameStateParseError(format!("Missing {} skills", category))
                })?;

            for (skill_name, skill_value) in category_obj {
                let value = skill_value.as_u64().ok_or_else(|| {
                    AIError::GameStateParseError(format!("Invalid skill value for {}", skill_name))
                })?;
                skills_map.insert(skill_name.clone(), value as u8);
            }
        }

        // Extract qualities
        let qualities = args
            .get("qualities")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
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
                    .collect::<Result<Vec<Quality>, AIError>>()
            })
            .unwrap_or_else(|| Ok(vec![]))?;

        self.add_debug_message("Character creation successful".to_string());

        // Create and return the CharacterSheet
        Ok(CharacterSheet::new(
            name, race, gender, backstory, body, agility, reaction, strength, willpower, logic,
            intuition, charisma, edge, magic, resonance, skills, qualities,
        ))
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
