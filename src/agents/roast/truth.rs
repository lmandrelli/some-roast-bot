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
    context: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let trigger = context.trigger_content();
    let preamble = crate::agents::preambles::select_preamble(
        crate::agents::preambles::ROAST_TRUTH,
        trigger,
        llm.magic_word(),
    );
    let mut intent = "Vérifie l'affirmation visée par le DÉCLENCHEUR et tranche.".to_string();
    if let Some(hint) = super::reply_hint(context) {
        intent = format!("{intent}\n{hint}");
    }
    try_roast_with_retry(llm, &preamble, &intent, context).await
}
