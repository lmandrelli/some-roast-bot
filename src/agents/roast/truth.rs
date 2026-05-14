use std::sync::Arc;

use rig::completion::Prompt;

use crate::agents::llm::LlmService;
use crate::bot::context::ChannelContext;
use crate::error::LlmError;

/// Respond to "is this true?" by judging the recent conversation.
pub async fn roast_truth(
    llm_service: &LlmService,
    ctx: Arc<poise::serenity_prelude::Context>,
    channel_id: poise::serenity_prelude::ChannelId,
    channel_ctx: &ChannelContext,
) -> Result<String, LlmError> {
    let context_str = channel_ctx.to_string();
    let prompt = format!(
        "{context_str}\n\nSomeone asked \"is this true?\". Judge the claim and roast accordingly."
    );

    let agent =
        llm_service.build_roast_agent(crate::agents::preambles::ROAST_TRUTH, ctx, channel_id);

    tracing::info!("Sending truth roast prompt to model...");
    let response = agent
        .prompt(&prompt)
        .max_turns(5)
        .await
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    tracing::info!("Truth roast response received: {} chars", response.len());
    Ok(response)
}
