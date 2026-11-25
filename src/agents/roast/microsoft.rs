use crate::agents::llm::LlmService;
use crate::agents::roast::{RoastOutput, try_roast_with_retry};
use crate::bot::context::ChannelContext;
use crate::db::MemoryRepository;
use crate::error::LlmError;

/// Roast when someone mentions Microsoft or Windows in a message.
/// Uses Exa web search to find latest Microsoft news and the memory database
/// to avoid repeating the same topics.
pub async fn roast_microsoft(
    llm_service: &LlmService,
    memory: &dyn MemoryRepository,
    author: &str,
    channel_ctx: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let past_topics = memory
        .recent_topics(10)
        .await
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    let topics_section = if past_topics.is_empty() {
        "Sujets déjà utilisés : aucun pour l'instant.\n".to_string()
    } else {
        let list = past_topics
            .iter()
            .enumerate()
            .map(|(i, t)| format!("  {}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n");
        format!("Sujets déjà utilisés (à ne pas reprendre) :\n{list}\n")
    };

    let trigger_content = channel_ctx.trigger_content();
    let magic_active =
        crate::agents::preambles::trigger_active(trigger_content, llm_service.magic_word());

    let base_preamble = crate::agents::preambles::select_preamble(
        crate::agents::preambles::ROAST_MICROSOFT,
        trigger_content,
        llm_service.magic_word(),
    );

    let preamble = format!("{base_preamble}\n{topics_section}");

    let prompt = if magic_active {
        format!(
            "{author} a parlé de Microsoft ou de Windows. \
             Réponds gentiment à son message, sans roaster personne."
        )
    } else {
        format!(
            "{author} a parlé de Microsoft ou de Windows. \
             Rappel : c'est « Microslop » et « Windaube », toujours."
        )
    };

    tracing::info!("Sending microsoft roast prompt to model...");
    let output = try_roast_with_retry(llm_service, &preamble, &prompt, channel_ctx).await?;

    if let Some(ref topic) = output.topic {
        tracing::info!("Storing microsoft news topic: {topic}");
        let _ = memory.remember_topic(topic).await;
    }

    Ok(output)
}
