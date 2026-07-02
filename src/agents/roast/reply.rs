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
    let trigger_content = channel_ctx.trigger.as_ref().map(|t| t.content.as_str());
    let magic_active =
        crate::agents::preambles::trigger_active(trigger_content, llm_service.magic_word());

    let prompt = if magic_active {
        format!(
            "{channel_ctx}\n\
             {tagger} tagged you in this conversation. \
             Respond kindly — do NOT roast anyone."
        )
    } else {
        format!(
            "{channel_ctx}\n\
             {tagger} tagged you to settle this. Roast whoever is wrong."
        )
    };

    let preamble = crate::agents::preambles::select_preamble(
        crate::agents::preambles::ROAST_REPLY,
        trigger_content,
        llm_service.magic_word(),
    );

    tracing::info!("Sending reply roast prompt to model...");
    try_roast_with_retry(llm_service, &preamble, &prompt).await
}
