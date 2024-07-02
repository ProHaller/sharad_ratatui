use crate::message::{Message, MessageType};
use async_openai::{
    config::OpenAIConfig,
    types::{
        AssistantTools, CreateAssistantRequestArgs, CreateMessageRequestArgs, CreateRunRequestArgs,
        CreateThreadRequestArgs, MessageContent, MessageRole, RunStatus,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use tokio::time::{timeout, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct GameConversationState {
    pub assistant_id: String,
    pub thread_id: String,
    pub player_health: u8,
    pub player_gold: u32,
    // Add other game-specific state here
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

pub struct GameAI {
    client: Client<OpenAIConfig>,
    pub conversation_state: Option<GameConversationState>,
}

impl GameAI {
    pub fn new(api_key: String) -> Result<Self, AIError> {
        let openai_config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(openai_config);

        Ok(Self {
            client,
            conversation_state: None,
        })
    }

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
                    .model("gpt-4-1106-preview")
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
            .content(format!(
                "Start a new game. Player health: {}, Player gold: {}",
                initial_game_state.player_health, initial_game_state.player_gold
            ))
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

        let mut run = self
            .client
            .threads()
            .runs(&state.thread_id)
            .create(run_request)
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

        let response = self.get_latest_message(&state.thread_id).await?;
        self.update_game_state(&response)?;
        Ok(response)
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
                    all_messages.push(Message {
                        content: text_content.text.value.clone(),
                        message_type: match message.role {
                            MessageRole::User => MessageType::User,
                            MessageRole::Assistant => MessageType::Game,
                            _ => MessageType::System,
                        },
                    });
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
        // This is a placeholder. You'll need to implement proper parsing based on your AI's response format.
        if let Some(state) = &mut self.conversation_state {
            // Example parsing, adjust according to your actual AI response format
            if let Some(health_str) = response.split("Player health:").nth(1) {
                if let Some(health) = health_str.split(',').next() {
                    state.player_health = health.trim().parse().map_err(|_| {
                        AIError::GameStateParseError("Failed to parse player health".into())
                    })?;
                }
            }
            if let Some(gold_str) = response.split("Player gold:").nth(1) {
                if let Some(gold) = gold_str.split('.').next() {
                    state.player_gold = gold.trim().parse().map_err(|_| {
                        AIError::GameStateParseError("Failed to parse player gold".into())
                    })?;
                }
            }
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
}
