use std::sync::Arc;

use rig::completion::Prompt;

use crate::agents::llm::LlmService;
use crate::bot::context::ChannelContext;
use crate::error::LlmError;

/// Roast based on recent channel messages.
/// The bot picks who to roast and mentions them.
pub async fn roast_channel(
    llm_service: &LlmService,
    ctx: Arc<poise::serenity_prelude::Context>,
    channel_id: poise::serenity_prelude::ChannelId,
    channel_ctx: &ChannelContext,
) -> Result<String, LlmError> {
    let context_str = channel_ctx.to_string();
    let prompt =
        format!("{context_str}\n\nPick someone to roast and tag them using their mention.");

    let agent =
        llm_service.build_roast_agent(crate::agents::preambles::ROAST_CHANNEL, ctx, channel_id);

    tracing::info!("Sending channel roast prompt to model...");
    let response = agent
        .prompt(&prompt)
        .max_turns(5)
        .await
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    tracing::info!("Channel roast response received: {} chars", response.len());
    Ok(response)
}

pub async fn roast_channel_with_context(
    llm_service: &LlmService,
    ctx: Arc<poise::serenity_prelude::Context>,
    channel_id: poise::serenity_prelude::ChannelId,
    context: &str,
) -> Result<String, LlmError> {
    let prompt = format!("{context}\n\nPick someone to roast and tag them using their mention.");

    let agent =
        llm_service.build_roast_agent(crate::agents::preambles::ROAST_CHANNEL, ctx, channel_id);

    tracing::info!("Sending channel roast retry prompt to model...");
    let response = agent
        .prompt(&prompt)
        .max_turns(5)
        .await
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    tracing::info!(
        "Channel roast retry response received: {} chars",
        response.len()
    );
    Ok(response)
}
