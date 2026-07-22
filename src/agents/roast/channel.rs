use crate::{
    agents::{
        llm::LlmService,
        roast::{RoastOutput, try_roast_with_retry},
    },
    bot::context::ChannelContext,
    error::LlmError,
};

pub async fn roast_channel(
    llm: &LlmService,
    context: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let trigger = context.trigger_content();
    let preamble = crate::agents::preambles::select_preamble(
        crate::agents::preambles::ROAST_GENERAL,
        trigger,
        llm.magic_word(),
    );
    let intent = if crate::agents::preambles::trigger_active(trigger, llm.magic_word()) {
        "The easter egg is active. Respond warmly to the trigger and do not roast."
    } else {
        "Inspect the trigger first and fulfill its roast intent. A target may be a guild member, external person, object, image, or idea. A mention or reply is context, not necessarily the target. If no target is requested, choose the most roastable subject from context."
    };
    try_roast_with_retry(llm, &preamble, intent, context).await
}
