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

    let trigger_content = channel_ctx.trigger.as_ref().map(|t| t.content.as_str());
    let preamble = crate::agents::preambles::select_preamble(
        crate::agents::preambles::ROAST_CHANNEL,
        trigger_content,
        llm_service.magic_word(),
    );

    tracing::info!("Sending channel roast prompt to model...");
    try_roast_with_retry(llm_service, &preamble, &prompt).await
}
