use crate::agents::llm::LlmService;
use crate::bot::context::{AuthorIdentity, ChannelContext};
use crate::error::LlmError;

/// Research a topic with web search.
pub async fn research(
    llm_service: &LlmService,
    asker: &AuthorIdentity,
    question: &str,
    context: &ChannelContext,
) -> Result<String, LlmError> {
    crate::agents::ask::answer(
        llm_service,
        crate::agents::preambles::RESEARCH,
        "/research",
        asker,
        question,
        context,
        6,
    )
    .await
}
