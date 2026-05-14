use rig::completion::Prompt;

use crate::agents::llm::LlmService;
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
) -> Result<String, LlmError> {
    // Fetch previously used topics from memory
    let past_topics = memory
        .recent_topics(20)
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

    let channel_formatted = channel_ctx.to_string();

    let full_preamble = format!(
        "{}\n{}\n---\nContext:\n",
        crate::agents::preambles::ROAST_MICROSOFT,
        topics_section
    );

    let prompt = format!(
        "{author} ({author_mention}) said: \"{message}\"\n\n\
         Channel context:\n\
         {channel_formatted}\n\n\
         They mentioned Microsoft or Windows. Destroy them. \
         Remember: it's \"Microslop\" and \"Windaube\", always. \
         Tag them using their mention: {author_mention}",
    );

    let agent = llm_service.build_search_agent(&full_preamble).await?;

    tracing::info!("Sending microsoft roast prompt to model...");
    let response = agent
        .prompt(&prompt)
        .max_turns(5)
        .await
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    tracing::info!(
        "Microsoft roast response received: {} chars",
        response.len()
    );

    // Extract and store the topic from the [TOPIC: ...] tag
    let (clean_response, topic) = extract_topic(&response);
    if let Some(topic) = topic {
        tracing::info!("Storing microsoft news topic: {topic}");
        let _ = memory.remember_topic(&topic);
    }

    Ok(clean_response)
}

/// Extract `[TOPIC: ...]` from the end of the response.
/// Returns (cleaned response, optional topic).
fn extract_topic(response: &str) -> (String, Option<String>) {
    if let Some(start) = response.rfind("[TOPIC:") {
        if let Some(end) = response[start..].find(']') {
            let topic = response[start + 7..start + end].trim().to_string();
            let clean = response[..start].trim().to_string();
            return (clean, Some(topic));
        }
    }
    (response.to_string(), None)
}
