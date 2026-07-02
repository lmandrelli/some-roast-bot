use rig::completion::Prompt;

use crate::agents::llm::LlmService;
use crate::error::LlmError;

/// Ask the AI a question with sarcastic roast flavor.
pub async fn ask(llm_service: &LlmService, question: &str) -> Result<String, LlmError> {
    let preamble = crate::agents::preambles::select_preamble(
        crate::agents::preambles::ASK,
        Some(question),
        llm_service.magic_word(),
    );

    let agent = llm_service.build_agent(&preamble).await?;

    tracing::info!("Sending /ask prompt to model...");
    let response = agent
        .prompt(question)
        .max_turns(2)
        .await
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    tracing::info!("/ask response received: {} chars", response.len());
    Ok(response)
}
