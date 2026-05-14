use rig::completion::Prompt;

use crate::agents::llm::LlmService;
use crate::bot::context::ChannelContext;
use crate::error::LlmError;

/// Roast when the bot is tagged in a reply to another message.
/// Settles the argument between the two users.
pub async fn roast_reply(
    llm_service: &LlmService,
    ctx: std::sync::Arc<poise::serenity_prelude::Context>,
    channel_id: poise::serenity_prelude::ChannelId,
    tagger: &str,
    tagger_mention: &str,
    tagger_msg: &str,
    target: &str,
    target_mention: &str,
    target_msg: &str,
    channel_ctx: &ChannelContext,
) -> Result<String, LlmError> {
    let channel_formatted = channel_ctx.to_string();

    let prompt = format!(
        "{tagger} ({tagger_mention}) said: \"{tagger_msg}\"\n\
         {target} ({target_mention}) said: \"{target_msg}\"\n\n\
         Channel context (what led to this):\n\
         {channel_formatted}\n\n\
         {tagger} tagged you to settle this. Roast whoever is wrong. \
         Tag them using their mention.",
    );

    let agent =
        llm_service.build_roast_agent(crate::agents::preambles::ROAST_REPLY, ctx, channel_id);

    tracing::info!("Sending reply roast prompt to model...");
    let response = agent
        .prompt(&prompt)
        .max_turns(5)
        .await
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    tracing::info!("Reply roast response received: {} chars", response.len());
    Ok(response)
}
