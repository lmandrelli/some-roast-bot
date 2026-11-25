use rig::completion::Prompt;

use crate::agents::llm::LlmService;
use crate::bot::context::{AuthorIdentity, ChannelContext};
use crate::error::LlmError;

/// Ask the AI a question with sarcastic roast flavor.
pub async fn ask(
    llm_service: &LlmService,
    asker: &AuthorIdentity,
    question: &str,
    context: &ChannelContext,
) -> Result<String, LlmError> {
    answer(
        llm_service,
        crate::agents::preambles::ASK,
        "/ask",
        asker,
        question,
        context,
        3,
    )
    .await
}

/// Answer a slash-command question with the recent channel messages (and
/// their images) as context.
pub(crate) async fn answer(
    llm_service: &LlmService,
    preamble: &str,
    command: &str,
    asker: &AuthorIdentity,
    question: &str,
    context: &ChannelContext,
    max_turns: usize,
) -> Result<String, LlmError> {
    let preamble = crate::agents::preambles::with_date(
        &crate::agents::preambles::select_preamble(
            preamble,
            Some(question),
            llm_service.magic_word(),
        ),
        chrono::Utc::now().date_naive(),
    );
    let (agent, _mcp_session) = llm_service.build_agent(&preamble).await?;
    let images = crate::agents::images::load_context_images(context).await;
    let text = format!(
        "{} (<@{}>) asks with {command}: {question}",
        asker.name, asker.id
    );

    tracing::info!("Sending {command} prompt to model...");
    let response = agent
        .prompt(crate::agents::prompt::prompt_message(
            text, context, &images,
        ))
        .max_turns(max_turns)
        .await
        .map_err(|e| LlmError::Completion(e.to_string()))?;

    tracing::info!("{command} response received: {} chars", response.len());
    Ok(response)
}
