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
    let mut intent = if crate::agents::preambles::trigger_active(trigger, llm.magic_word()) {
        "L'easter egg est actif : réponds chaleureusement au DÉCLENCHEUR, sans roaster personne."
            .to_string()
    } else {
        "Roaste ce que demande le DÉCLENCHEUR.".to_string()
    };
    if let Some(hint) = super::reply_hint(context) {
        intent = format!("{intent}\n{hint}");
    }
    try_roast_with_retry(llm, &preamble, &intent, context).await
}
