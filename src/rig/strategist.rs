use rig::{
    agent::Agent,
    client::{CompletionClient, ProviderClient},
    completion::ToolDefinition,
    providers::openai::{CompletionModel, GPT_4O_MINI},
    tool::{Tool, ToolEmbedding},
};

use super::{CHRUNCHER_PREAMBLE, CrunchCall, STRATEGIST_PREAMBLE};

pub fn build_strategist_with_cruncher() -> Agent<CompletionModel> {
    let openai_client = rig::providers::openai::Client::from_env();

    openai_client
        .agent(GPT_4O_MINI)
        .preamble(STRATEGIST_PREAMBLE)
        .tool(CrunchCall {
            name: "cruncher_call".to_string(),
            description: "Agent that handles the crunch and tool call to the game state".into(),
            agent_preamble: CHRUNCHER_PREAMBLE.into(),
        })
        .build()
}
