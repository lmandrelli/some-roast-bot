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
    channel_ctx: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let prompt = format!(
        "{channel_ctx}\n\
         {tagger} wants you to roast {target}. \
         Roast {target} based on what they said in the conversation."
    );

    tracing::info!("Sending user roast prompt to model...");
    try_roast_with_retry(llm_service, crate::agents::preambles::ROAST_USER, &prompt).await
}
