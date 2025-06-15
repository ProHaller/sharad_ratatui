// TODO: Create the Narrator from a json asset
// TODO: Ensure the Crunch, fluff and Dialogue format
// TODO: Prepare a implementation of streaming responses

use rig::{
    agent::Agent,
    client::{CompletionClient, ProviderClient},
    providers::openai::{CompletionModel, GPT_4O_MINI},
};

use super::NARRATOR_PREAMBLE;

pub fn build_strategist_with_cruncher() -> Agent<CompletionModel> {
    let openai_client = rig::providers::openai::Client::from_env();

    openai_client
        .agent(GPT_4O_MINI)
        .preamble(NARRATOR_PREAMBLE)
        .build()
}
