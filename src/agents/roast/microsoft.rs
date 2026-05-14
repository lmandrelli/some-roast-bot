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
    author_mention: &str,
    message: &str,
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

    let preamble = format!(
        "{}\n{}\n---\nContext:\n",
        crate::agents::preambles::ROAST_MICROSOFT,
        topics_section
    );

    let prompt = format!(
        "{} ({}) said: \"{}\"\n\n\
         Channel context:\n\
         {}\n\n\
         They mentioned Microsoft or Windows. Destroy them. \
         Remember: it's \"Microslop\" and \"Windaube\", always.",
        author,
        author_mention,
        message,
        channel_ctx.to_string()
    );

    tracing::info!("Sending microsoft roast prompt to model...");
    let output = try_roast_with_retry(llm_service, &preamble, &prompt).await?;

    if let Some(ref topic) = output.topic {
        tracing::info!("Storing microsoft news topic: {topic}");
        let _ = memory.remember_topic(topic);
    }

    Ok(output)
}
