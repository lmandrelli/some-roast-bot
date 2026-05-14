use crate::agents::llm::LlmService;
use crate::agents::roast::{RoastOutput, try_roast_with_retry};
use crate::bot::context::ChannelContext;
use crate::error::LlmError;

/// Roast when the bot is tagged alongside another user.
/// Analyzes the target user's recent messages and roasts them.
pub async fn roast_user(
    llm_service: &LlmService,
    tagger: &str,
    target: &str,
    target_mention: &str,
    channel_ctx: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let prompt = format!(
        "{tagger} wants you to roast {target} ({target_mention}).\n\n\
         Channel context:\n\
         {}\n\n\
         Roast {target} based on what they said in the conversation.",
        channel_ctx.to_string()
    );

    tracing::info!("Sending user roast prompt to model...");
    try_roast_with_retry(llm_service, crate::agents::preambles::ROAST_USER, &prompt).await
}
