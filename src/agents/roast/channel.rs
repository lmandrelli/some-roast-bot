use crate::agents::llm::LlmService;
use crate::agents::roast::{RoastOutput, try_roast_with_retry};
use crate::bot::context::ChannelContext;
use crate::error::LlmError;

/// Roast based on recent channel messages.
/// The bot picks who to roast and mentions them.
pub async fn roast_channel(
    llm_service: &LlmService,
    channel_ctx: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let prompt = format!("{}\n\nPick someone to roast.", channel_ctx.to_string());

    tracing::info!("Sending channel roast prompt to model...");
    try_roast_with_retry(
        llm_service,
        crate::agents::preambles::ROAST_CHANNEL,
        &prompt,
    )
    .await
}
