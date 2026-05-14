use crate::agents::llm::LlmService;
use crate::agents::roast::{RoastOutput, try_roast_with_retry};
use crate::bot::context::ChannelContext;
use crate::error::LlmError;

/// Roast when the bot is tagged in a reply to another message.
/// Settles the argument between the two users.
pub async fn roast_reply(
    llm_service: &LlmService,
    tagger: &str,
    _target: &str,
    channel_ctx: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let prompt = format!(
        "{channel_ctx}\n\
         {tagger} tagged you to settle this. Roast whoever is wrong."
    );

    tracing::info!("Sending reply roast prompt to model...");
    try_roast_with_retry(llm_service, crate::agents::preambles::ROAST_REPLY, &prompt).await
}
