mod channel;
mod microsoft;
mod reply;
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

async fn call_model(
    preamble: &str,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let openai_client = CompletionsClient::from_env();
    let model = openai_client.completion_model("moonshotai/Kimi-K2.5-TEE");

    let agent = rig::agent::AgentBuilder::new(model)
        .preamble(preamble)
        .build();

    tracing::info!("Sending roast prompt to model...");
    let response = agent
        .prompt(prompt)
        .await
        .inspect_err(|e| tracing::error!("Roast completion error: {:?}", e))?;
    tracing::info!("Roast response received: {} chars", response.len());
    Ok(response)
}
