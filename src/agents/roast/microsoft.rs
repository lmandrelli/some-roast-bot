use crate::agents::llm::LlmService;
use crate::agents::roast::{RoastOutput, try_roast_with_retry};
use crate::bot::context::ChannelContext;
use crate::db::MemoryRepository;
use crate::error::LlmError;

/// Roast when someone mentions Microsoft or Windows in a message.
/// Uses Exa web search to find latest Microsoft news and SQLite memory
/// to avoid repeating the same topics.
pub async fn roast_microsoft(
    llm_service: &LlmService,
    memory: &dyn MemoryRepository,
    author: &str,
    channel_ctx: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let past_topics = memory
        .recent_topics(10)
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    let topics_section = if past_topics.is_empty() {
        "Already Used Topics: (none yet)\n".to_string()
    } else {
        let list = past_topics
            .iter()
            .enumerate()
            .map(|(i, t)| format!("  {}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n");
        format!("Already Used Topics (DO NOT reuse these):\n{list}\n")
    };

    let trigger_content = channel_ctx.trigger.as_ref().map(|t| t.content.as_str());
    let magic_active =
        crate::agents::preambles::trigger_active(trigger_content, llm_service.magic_word());

    let base_preamble = crate::agents::preambles::select_preamble(
        crate::agents::preambles::ROAST_MICROSOFT,
        trigger_content,
        llm_service.magic_word(),
    );

    let preamble = format!("{}\n{}\n---\nContext:\n", base_preamble, topics_section);

    let prompt = if magic_active {
        format!(
            "{channel_ctx}\n\
             {author} mentioned Microsoft or Windows. \
             Respond to their message nicely — do NOT roast anyone."
        )
    } else {
        format!(
            "{channel_ctx}\n\
             {author} mentioned Microsoft or Windows. Destroy them. \
             Remember: it's \"Microslop\" and \"Windaube\", always."
        )
    };

    tracing::info!("Sending microsoft roast prompt to model...");
    let output = try_roast_with_retry(llm_service, &preamble, &prompt).await?;

    if let Some(ref topic) = output.topic {
        tracing::info!("Storing microsoft news topic: {topic}");
        let _ = memory.remember_topic(topic);
    }

    Ok(output)
}
