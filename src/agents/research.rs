use rig::completion::Prompt;

use crate::agents::llm::LlmService;
use crate::error::LlmError;

/// Research a topic with web search.
pub async fn research(llm_service: &LlmService, question: &str) -> Result<String, LlmError> {
    let agent = llm_service
        .build_search_agent(crate::agents::preambles::RESEARCH)
        .await?;

    tracing::info!("Sending /research prompt to model...");
    let response = agent
        .prompt(question)
        .max_turns(4)
        .await
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    tracing::info!("/research response received: {} chars", response.len());
    Ok(response)
}
