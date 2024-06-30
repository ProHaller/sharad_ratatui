use async_openai::{
    config::OpenAIConfig,
    types::{
        AssistantTools, AssistantToolsFunction, CreateMessageRequestArgs, CreateRunRequestArgs,
        CreateThreadRequestArgs, FunctionObject, MessageContent, MessageObject, MessageRole,
        RunObject, RunStatus, SubmitToolOutputsRunRequest, ToolsOutputs,
    },
    Client,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ConversationState {
    pub assistant_id: String,
    pub thread_id: String,
}

pub struct AI {
    client: Client<OpenAIConfig>,
    pub conversation_state: Option<ConversationState>,
}

impl AI {
    pub fn new(api_key: String) -> Result<Self, Box<dyn std::error::Error>> {
        let openai_config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(openai_config);

        Ok(Self {
            client,
            conversation_state: None,
        })
    }

    pub async fn start_new_conversation(
        &mut self,
        assistant_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let thread = self
            .client
            .threads()
            .create(CreateThreadRequestArgs::default().build()?)
            .await?;

        self.conversation_state = Some(ConversationState {
            assistant_id: assistant_id.to_string(),
            thread_id: thread.id.clone(),
        });

        let initial_message = CreateMessageRequestArgs::default()
            .role(MessageRole::User)
            .content("Start the conversation.")
            .build()?;
        self.client
            .threads()
            .messages(&thread.id)
            .create(initial_message)
            .await?;

        Ok(())
    }

    pub async fn continue_conversation(&mut self, conversation_state: ConversationState) {
        self.conversation_state = Some(conversation_state);
    }

    pub async fn send_message(&self, message: &str) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(state) = &self.conversation_state {
            let message_request = CreateMessageRequestArgs::default()
                .role(MessageRole::User)
                .content(message)
                .build()?;
            self.client
                .threads()
                .messages(&state.thread_id)
                .create(message_request)
                .await?;
            let response = self.get_latest_message(&state.thread_id).await?;
            Ok(response)
        } else {
            Err("Conversation state is not initialized".into())
        }
    }

    async fn get_latest_message(
        &self,
        thread_id: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
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
        Err("No message found".into())
    }
}
