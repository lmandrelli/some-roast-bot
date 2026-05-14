use rig::completion::Prompt;

use crate::agents::llm::LlmService;
use crate::error::LlmError;

/// Ask the AI a question with sarcastic roast flavor.
pub async fn ask(llm_service: &LlmService, question: &str) -> Result<String, LlmError> {
    let agent = llm_service
        .build_search_agent(crate::agents::preambles::ASK)
        .await?;

    tracing::info!("Sending /ask prompt to model...");
    let response = agent
        .prompt(question)
        .max_turns(2)
        .await
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    tracing::info!("/ask response received: {} chars", response.len());
    Ok(response)
}
