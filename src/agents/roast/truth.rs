use crate::agents::llm::LlmService;
use crate::agents::roast::{RoastOutput, try_roast_with_retry};
use crate::bot::context::ChannelContext;
use crate::error::LlmError;

/// Respond to "is this true?" by judging the recent conversation.
pub async fn roast_truth(
    llm_service: &LlmService,
    channel_ctx: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let trigger_content = channel_ctx.trigger.as_ref().map(|t| t.content.as_str());
    let magic_active =
        crate::agents::preambles::trigger_active(trigger_content, llm_service.magic_word());

    let prompt = if magic_active {
        format!(
            "{channel_ctx}\n\nSomeone asked \"is this true?\". Answer the question helpfully and kindly — do NOT roast anyone."
        )
    } else {
        format!(
            "{channel_ctx}\n\nSomeone asked \"is this true?\". Judge the claim and roast accordingly."
        )
    };

    let preamble = crate::agents::preambles::select_preamble(
        crate::agents::preambles::ROAST_TRUTH,
        trigger_content,
        llm_service.magic_word(),
    );

    tracing::info!("Sending truth roast prompt to model...");
    try_roast_with_retry(llm_service, &preamble, &prompt).await
}
