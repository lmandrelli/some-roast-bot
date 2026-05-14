use crate::agents::llm::LlmService;
use crate::agents::roast::{RoastOutput, try_roast_with_retry};
use crate::bot::context::ChannelContext;
use crate::error::LlmError;

/// Roast when the bot is tagged in a reply to another message.
/// Settles the argument between the two users.
pub async fn roast_reply(
    llm_service: &LlmService,
    tagger: &str,
    tagger_mention: &str,
    tagger_msg: &str,
    target: &str,
    target_mention: &str,
    target_msg: &str,
    channel_ctx: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let prompt = format!(
        "{tagger} ({tagger_mention}) said: \"{tagger_msg}\"\n\
         {target} ({target_mention}) said: \"{target_msg}\"\n\n\
         Channel context (what led to this):\n\
         {}\n\n\
         {tagger} tagged you to settle this. Roast whoever is wrong.",
        channel_ctx.to_string()
    );

    tracing::info!("Sending reply roast prompt to model...");
    try_roast_with_retry(llm_service, crate::agents::preambles::ROAST_REPLY, &prompt).await
}
