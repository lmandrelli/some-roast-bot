use crate::agents::llm::LlmService;
use crate::agents::roast::{RoastOutput, try_roast_with_retry};
use crate::bot::context::ChannelContext;
use crate::error::LlmError;

/// Respond to "is this true?" by judging the recent conversation.
pub async fn roast_truth(
    llm_service: &LlmService,
    channel_ctx: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let prompt = format!(
        "{}\n\nSomeone asked \"is this true?\". Judge the claim and roast accordingly.",
        channel_ctx.to_string()
    );

    tracing::info!("Sending truth roast prompt to model...");
    try_roast_with_retry(llm_service, crate::agents::preambles::ROAST_TRUTH, &prompt).await
}
