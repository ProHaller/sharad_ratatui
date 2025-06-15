// TODO: Create the Strategist definition from an asset json
// TODO: Create the cruncher_call tool
// TODO: Create a helper function to concatenate [User message, history, character information and
// memory]

use rig::{
    client::completion::CompletionClient,
    completion::Prompt,
    pipeline::Op,
    providers::openai::{Client, GPT_4O},
    tool::Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

// Define the tool that will make an agent call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrunchCall {
    pub name: String,
    pub description: String,
    pub agent_preamble: String,
}

// Tool parameters for the agent call
#[derive(Debug, Serialize, Deserialize)]
pub struct CrunchCallParams {
    pub query: String,
    pub context: Option<String>,
}

// Tool result from the agent call
#[derive(Debug, Serialize, Deserialize)]
pub struct CrunchCallResult {
    pub response: String,
    pub success: bool,
    pub metadata: Option<String>,
}

// Create a custom error type for the agent call tool
#[derive(Debug, thiserror::Error)]
pub enum CrunchError {
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("Agent prompt error: {0}")]
    Prompt(String),
    #[error("General error: {0}")]
    General(String),
}

impl Tool for CrunchCall {
    const NAME: &'static str = "cruncher_call";

    type Error = CrunchError;
    type Args = CrunchCallParams;
    type Output = CrunchCallResult;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The query or task to send to the agent"
                    },
                    "context": {
                        "type": "string",
                        "description": "Additional context for the agent (optional)"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let cloned = self.clone();
        let result = tokio::spawn(async move {
            // Create OpenAI client
            let client = Client::new(&std::env::var("OPENAI_API_KEY").unwrap());

            // Build the agent with the specified prompt
            let agent = client.agent(GPT_4O).preamble(&cloned.agent_preamble).build();

            // Prepare the full query with context if provided
            let full_query = if let Some(context) = args.context {
                format!("Context: {}\n\nQuery: {}", context, args.query)
            } else {
                args.query
            };
            // Make the agent call
            match agent.prompt(&full_query).await {
                Ok(response) => Ok::<CrunchCallResult, String>(CrunchCallResult {
                    response,
                    success: true,
                    metadata: Some("Agent call completed successfully, but the tool hasn't yet been implemented, simulate the response result to continue the test.".to_string()),
                }),
                Err(e) => Ok(CrunchCallResult {
                    response: format!("Error: {}", e),
                    success: false,
                    metadata: Some("Agent call failed".to_string()),
                }),
            }
        })
        .await
        .map_err(|e| CrunchError::General(e.to_string()))?
        .unwrap();
        Ok(result)
    }
}

// Pipeline operation that uses the agent call tool
// Note: Op trait doesn't have Error associated type, so we handle errors internally
#[derive(Debug)]
pub struct AgentCallOp {
    pub agent_tool: CrunchCall,
}

impl Op for AgentCallOp {
    type Input = String;
    type Output = Result<String, String>; // Changed to Result to handle errors

    async fn call(&self, input: Self::Input) -> Self::Output {
        let params = CrunchCallParams {
            query: input,
            context: None,
        };

        match self.agent_tool.call(params).await {
            Ok(result) => {
                if result.success {
                    Ok(result.response)
                } else {
                    Err(format!("Agent call failed: {}", result.response))
                }
            }
            Err(e) => Err(format!("Agent call error: {}", e)),
        }
    }
}

// Alternative implementation that doesn't use Result in Output
#[derive(Debug)]
pub struct CrunchCallOp {
    pub agent_tool: CrunchCall,
}

impl Op for CrunchCallOp {
    type Input = String;
    type Output = String;

    async fn call(&self, input: Self::Input) -> Self::Output {
        let params = CrunchCallParams {
            query: input,
            context: None,
        };

        match self.agent_tool.call(params).await {
            Ok(result) => {
                if result.success {
                    result.response
                } else {
                    format!("Agent call failed: {}", result.response)
                }
            }
            Err(e) => format!("Agent call error: {}", e),
        }
    }
}
