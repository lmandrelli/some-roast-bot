use crate::{
    agents::{
        llm::LlmService,
        roast::{RoastOutput, try_roast_with_retry},
    },
    bot::context::ChannelContext,
    error::LlmError,
};
pub async fn roast_truth(
    llm: &LlmService,
    memory: &dyn crate::db::MemoryRepository,
    context: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let trigger = context.trigger_content();
    let preamble = crate::agents::preambles::select_preamble(
        crate::agents::preambles::ROAST_TRUTH,
        trigger,
        llm.magic_word(),
    );
    try_roast_with_retry(
        llm,
        memory,
        &preamble,
        "Verify and answer the claim requested by the trigger.",
        context,
    )
    .await
}
