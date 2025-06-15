// TODO: Create the preamble from an archivist asset
// TODO: Create the tools: add_memory, remove_memory,
// TODO: Create the memory Vector Store

use rig::{
    agent::Agent,
    client::{CompletionClient, ProviderClient},
    completion::ToolDefinition,
    providers::openai::{CompletionModel, GPT_4O_MINI},
    tool::{Tool, ToolEmbedding},
    vector_store::VectorStoreIndexDyn,
};

use super::ARCHIVIST_PREAMBLE;

pub fn build_archivist_with_dyn_context(
    index: impl VectorStoreIndexDyn + 'static,
) -> Agent<CompletionModel> {
    let openai_client = rig::providers::openai::Client::from_env();

    openai_client
        .agent(GPT_4O_MINI)
        .preamble(ARCHIVIST_PREAMBLE)
        .dynamic_context(5, index) // Increased to 4 since we have chunks now
        .tool(AddMemory)
        .tool(RemoveMemory)
        .build()
}

#[derive(serde::Deserialize)]
pub struct Memory {
    title: String,
    content: String,
    tags: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("Memory error")]
pub struct MemoryError;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct AddMemory;

impl Tool for AddMemory {
    const NAME: &'static str = "add_memory";

    type Error = MemoryError;
    type Args = Memory;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "add_memory".to_string(),
            description: "Add a single atomic memory to the Game Master's long-term memory. Each call MUST contain only **one** piece of information on **one** subject. If the input contains several facts, you MUST call this tool multiple times — once per fact — with appropriately split content and tags.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Short description of the memory"
                    },
                    "content": {
                        "type": "string",
                        "description": "Exactly **one** factual statement. Never combine multiple facts into one memory."
                    },
                    "tags": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "list of tags to classify the memory"
                    }
                },
                "required": ["title", "content", "tags"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = format!(
            "Title:\n{}\nTags:{:#?}\nContent:\n{}",
            args.title, args.tags, args.content
        );
        println!("{:#?}", result);
        Ok(result)
    }
}

impl ToolEmbedding for AddMemory {
    type InitError = MemoryError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(AddMemory)
    }

    fn context(&self) -> Self::Context {}

    fn embedding_docs(&self) -> Vec<String> {
        vec!["Add an atomic memory to the Game Master Long Term Memory".into()]
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct RemoveMemory;

impl Tool for RemoveMemory {
    const NAME: &'static str = "remove_memory";

    type Error = MemoryError;
    type Args = String;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "remove_memory".to_string(),
            description: "Remove an atomic piece of memory from the Game Master long term memory"
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Exact title of the memory"
                    }
                },
                "required": ["title"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = format!("Memory removed: {}", args);
        println!("{:#?}", result);
        Ok(result)
    }
}

impl ToolEmbedding for RemoveMemory {
    type InitError = MemoryError;
    type Context = ();
    type State = ();

    fn init(_state: Self::State, _context: Self::Context) -> Result<Self, Self::InitError> {
        Ok(RemoveMemory)
    }

    fn context(&self) -> Self::Context {}

    fn embedding_docs(&self) -> Vec<String> {
        vec!["Remove an atomic memory From the Game Master Long Term Memory".into()]
    }
}
