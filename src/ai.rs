use crate::character::{CharacterSheet, Quality};
use crate::message::{GameMessage, Message, MessageType};
use async_openai::{
    config::OpenAIConfig,
    types::{
        AssistantTools, AssistantToolsFunction, CreateAssistantRequestArgs,
        CreateMessageRequestArgs, CreateRunRequestArgs, CreateThreadRequestArgs, FunctionObject,
        MessageContent, MessageRole, RunObject, RunStatus, ToolsOutputs,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::TryInto;
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

            self.add_debug_message(format!("Run status: {:?}", run.status));

            match run.status {
                RunStatus::Completed => {
                    self.add_debug_message("Run completed successfully".to_string());
                    return Ok(());
                }
                RunStatus::Failed | RunStatus::Cancelled | RunStatus::Expired => {
                    self.add_debug_message(format!("Run failed with status: {:?}", run.status));
                    return Err(AIError::GameStateParseError(format!(
                        "Run failed with status: {:?}",
                        run.status
                    )));
                }
                RunStatus::RequiresAction => {
                    self.add_debug_message(
                        "Run requires action, handling function call".to_string(),
                    );
                    self.handle_function_call(thread_id).await?;
                }
                _ => {
                    self.add_debug_message("Run still in progress, waiting...".to_string());
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
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
                self.add_debug_message(text_content.text.value.clone());
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

    async fn handle_function_call(&self, thread_id: &str) -> Result<(), AIError> {
        let message = self.get_latest_message(thread_id).await?;
        let json_message: serde_json::Value = serde_json::from_str(&message)?;

        if let Some(function_call) = json_message.get("function_call") {
            let function_name = function_call
                .get("name")
                .ok_or(AIError::NoMessageFound)?
                .as_str()
                .unwrap();
            let function_args = function_call
                .get("arguments")
                .ok_or(AIError::NoMessageFound)?;

            match function_name {
                "create_character_sheet" => {
                    let character_sheet = self.create_character(function_args).await?;
                    self.submit_function_output(
                        thread_id,
                        "create_character_sheet",
                        &character_sheet,
                    )
                    .await?;
                    Ok(())
                }
                _ => Err(AIError::GameStateParseError(format!(
                    "Unknown function: {}",
                    function_name
                ))),
            }
        } else {
            // Attempt to extract the function call from within the message text
            if let Some(reasoning) = json_message.get("reasoning") {
                if reasoning.as_str().unwrap_or("").contains("function call") {
                    let function_call_text = reasoning
                        .as_str()
                        .unwrap_or("")
                        .split("function call")
                        .nth(1)
                        .unwrap_or("")
                        .trim();
                    let function_call_json: serde_json::Value =
                        serde_json::from_str(function_call_text)?;

                    if let Some(function_call) = function_call_json.get("function_call") {
                        let function_name = function_call
                            .get("name")
                            .ok_or(AIError::NoMessageFound)?
                            .as_str()
                            .unwrap();
                        let function_args = function_call
                            .get("arguments")
                            .ok_or(AIError::NoMessageFound)?;

                        match function_name {
                            "create_character_sheet" => {
                                let character_sheet = self.create_character(function_args).await?;
                                self.submit_function_output(
                                    thread_id,
                                    "create_character_sheet",
                                    &character_sheet,
                                )
                                .await?;
                                Ok(())
                            }
                            _ => Err(AIError::GameStateParseError(format!(
                                "Unknown function: {}",
                                function_name
                            ))),
                        }
                    } else {
                        Err(AIError::GameStateParseError(
                            "No function call found".to_string(),
                        ))
                    }
                } else {
                    Err(AIError::GameStateParseError(
                        "No function call found".to_string(),
                    ))
                }
            } else {
                Err(AIError::GameStateParseError(
                    "No function call found".to_string(),
                ))
            }
        }
    }

    pub async fn submit_function_output(
        &self,
        thread_id: &str,
        function_name: &str,
        output: &impl Serialize,
    ) -> Result<(), AIError> {
        let tool_output = serde_json::to_string(output)
            .map_err(|e| AIError::GameStateParseError(e.to_string()))?;

        let submit_request = async_openai::types::SubmitToolOutputsRunRequest {
            tool_outputs: vec![ToolsOutputs {
                tool_call_id: Some(function_name.to_string()),
                output: Some(tool_output),
            }],
            stream: None,
        };

        self.client
            .threads()
            .runs(thread_id)
            .submit_tool_outputs(thread_id, submit_request)
            .await?;

        Ok(())
    }

    async fn create_character(&self, args: &serde_json::Value) -> Result<CharacterSheet, AIError> {
        fn extract_str_field<'a>(
            args: &'a serde_json::Value,
            field: &str,
        ) -> Result<&'a str, AIError> {
            args.get(field)
                .ok_or_else(|| {
                    AIError::GameStateParseError(format!("Missing '{}' argument", field))
                })?
                .as_str()
                .ok_or_else(|| {
                    AIError::GameStateParseError(format!("'{}' should be a string", field))
                })
        }

        fn extract_i64_field(args: &serde_json::Value, field: &str) -> Result<u8, AIError> {
            args.get(field)
                .ok_or_else(|| {
                    AIError::GameStateParseError(format!("Missing '{}' attribute", field))
                })?
                .as_i64()
                .ok_or_else(|| {
                    AIError::GameStateParseError(format!("'{}' should be an integer", field))
                })
                .and_then(|v| {
                    v.try_into().map_err(|_| {
                        AIError::GameStateParseError(format!("'{}' out of range", field))
                    })
                })
        }

        let name = extract_str_field(args, "name")?.to_string();
        let race = extract_str_field(args, "race")?.to_string();
        let gender = extract_str_field(args, "gender")?.to_string();
        let backstory = extract_str_field(args, "backstory")?.to_string();

        let attributes = args.get("attributes").ok_or(AIError::GameStateParseError(
            "Missing 'attributes' argument".to_string(),
        ))?;
        let body = extract_i64_field(attributes, "body")?;
        let agility = extract_i64_field(attributes, "agility")?;
        let reaction = extract_i64_field(attributes, "reaction")?;
        let strength = extract_i64_field(attributes, "strength")?;
        let willpower = extract_i64_field(attributes, "willpower")?;
        let logic = extract_i64_field(attributes, "logic")?;
        let intuition = extract_i64_field(attributes, "intuition")?;
        let charisma = extract_i64_field(attributes, "charisma")?;
        let edge = extract_i64_field(attributes, "edge")?;
        let magic = attributes.get("magic").map_or(Ok(0), |v| {
            v.as_i64()
                .unwrap_or(0)
                .try_into()
                .map_err(|_| AIError::GameStateParseError("'magic' out of range".to_string()))
        })?;
        let resonance = attributes.get("resonance").map_or(Ok(0), |v| {
            v.as_i64()
                .unwrap_or(0)
                .try_into()
                .map_err(|_| AIError::GameStateParseError("'resonance' out of range".to_string()))
        })?;

        let skills = args
            .get("skills")
            .ok_or(AIError::GameStateParseError(
                "Missing 'skills' argument".to_string(),
            ))?
            .as_object()
            .ok_or(AIError::GameStateParseError(
                "'skills' should be an object".to_string(),
            ))?;
        let skill_map = skills
            .iter()
            .map(|(k, v)| {
                v.as_i64()
                    .ok_or(AIError::GameStateParseError(format!(
                        "'{}' skill value should be an integer",
                        k
                    )))
                    .and_then(|val| {
                        val.try_into()
                            .map_err(|_| {
                                AIError::GameStateParseError(format!(
                                    "'{}' skill value out of range",
                                    k
                                ))
                            })
                            .map(|v| (k.clone(), v))
                    })
            })
            .collect::<Result<std::collections::HashMap<_, _>, _>>()?;

        let qualities = args
            .get("qualities")
            .ok_or(AIError::GameStateParseError(
                "Missing 'qualities' argument".to_string(),
            ))?
            .as_array()
            .ok_or(AIError::GameStateParseError(
                "'qualities' should be an array".to_string(),
            ))?;
        let quality_list = qualities
            .iter()
            .map(|quality| {
                let name = quality
                    .get("name")
                    .ok_or(AIError::GameStateParseError(
                        "Missing 'name' in quality".to_string(),
                    ))?
                    .as_str()
                    .ok_or(AIError::GameStateParseError(
                        "'name' in quality should be a string".to_string(),
                    ))?
                    .to_string();
                let positive = quality
                    .get("positive")
                    .ok_or(AIError::GameStateParseError(
                        "Missing 'positive' in quality".to_string(),
                    ))?
                    .as_bool()
                    .ok_or(AIError::GameStateParseError(
                        "'positive' in quality should be a boolean".to_string(),
                    ))?;
                Ok(Quality { name, positive })
            })
            .collect::<Result<Vec<Quality>, AIError>>()?;

        Ok(CharacterSheet::new(
            name,
            race,
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
            magic,
            resonance,
            skill_map,
            quality_list,
        ))
    }
}
