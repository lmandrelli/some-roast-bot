use std::sync::Arc;

use rig::completion::Prompt;

use crate::agents::llm::LlmService;
use crate::bot::context::{ChannelContext, FormattedMessage};
use crate::error::LlmError;

/// Roast when the bot is tagged alongside another user.
/// Analyzes the target user's recent messages and roasts them.
pub async fn roast_user(
    llm_service: &LlmService,
    ctx: Arc<poise::serenity_prelude::Context>,
    channel_id: poise::serenity_prelude::ChannelId,
    tagger: &str,
    target: &str,
    target_mention: &str,
    target_messages: &[FormattedMessage],
    channel_ctx: &ChannelContext,
) -> Result<String, LlmError> {
    let target_formatted = crate::bot::context::format_user_messages(target_messages);
    let channel_formatted = channel_ctx.to_string();

    let prompt = format!(
        "{tagger} wants you to roast {target} ({target_mention}).\n\n\
         {target}'s recent messages:\n\
         {target_formatted}\n\n\
         Channel context:\n\
         {channel_formatted}\n\n\
         Tag them using their mention: {target_mention}",
    );

    let agent =
        llm_service.build_roast_agent(crate::agents::preambles::ROAST_USER, ctx, channel_id);

    tracing::info!("Sending user roast prompt to model...");
    let response = agent
        .prompt(&prompt)
        .max_turns(5)
        .await
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    tracing::info!("User roast response received: {} chars", response.len());
    Ok(response)
}
