mod channel;
mod microsoft;
mod reply;
mod tools;
mod truth;
mod user;

pub use channel::roast_channel;
pub use microsoft::roast_microsoft;
pub use reply::roast_reply;
pub use truth::roast_truth;
pub use user::roast_user;

use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai::CompletionsClient;
use std::sync::Arc;

async fn call_model_with_tools(
    preamble: &str,
    prompt: &str,
    ctx: Arc<poise::serenity_prelude::Context>,
    channel_id: poise::serenity_prelude::ChannelId,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let model_name = crate::agents::model_name();
    let openai_client = CompletionsClient::from_env();
    let model = openai_client.completion_model(&model_name);

    let fetch_tool = tools::FetchMessagesTool {
        ctx,
        channel_id,
    };

    let agent = rig::agent::AgentBuilder::new(model)
        .preamble(preamble)
        .tool(fetch_tool)
        .build();

    tracing::info!("Sending roast prompt to model with tools ({model_name})...");
    let response = agent
        .prompt(prompt)
        .max_turns(5)
        .await
        .inspect_err(|e| tracing::error!("Roast completion error: {:?}", e))?;
    tracing::info!("Roast response received: {} chars", response.len());
    Ok(response)
}
