mod channel;
mod microsoft;
mod reply;
mod truth;
mod user;

pub use channel::roast_channel;
pub use microsoft::roast_microsoft;
pub use reply::roast_reply;
pub use truth::roast_truth;
pub use user::roast_user;

use rig::completion::Prompt;

use crate::agents::llm::LlmService;
use crate::error::LlmError;

/// Structured output from any roast agent.
#[derive(Debug, Clone)]
pub struct RoastOutput {
    pub mention_id: String,
    pub roast: String,
    pub topic: Option<String>,
}

/// Call an agent with the given preamble and prompt, retrying up to 2
/// times if the response does not contain valid XML.  On retry the
/// LLM receives its previous response plus the parse error so it can
/// correct itself.
pub async fn try_roast_with_retry(
    llm_service: &LlmService,
    preamble: &str,
    prompt: &str,
) -> Result<RoastOutput, LlmError> {
    let agent = llm_service.build_agent(preamble).await?;

    let mut previous_response = String::new();
    let mut last_error = String::new();

    for attempt in 0..3 {
        let full_prompt = if last_error.is_empty() {
            prompt.to_string()
        } else {
            format!(
                "{prompt}\n\n---\nPARSE ERROR — your previous response was invalid and could not be processed.\n\nYour previous response:\n```\n{previous_response}\n```\n\nWhy it failed: {last_error}\n\nYou MUST fix this exact issue and output ONLY valid XML in the exact format specified in your instructions."
            )
        };

        let response = agent
            .prompt(&full_prompt)
            .max_turns(4)
            .await
            .map_err(|e| LlmError::Completion(e.to_string()))?;

        match parse_roast_response(&response) {
            Ok(output) => return Ok(output),
            Err(e) => {
                tracing::warn!("Roast parse failed (attempt {}): {}", attempt + 1, e);
                previous_response = response;
                last_error = e.to_string();
            }
        }
    }

    Err(LlmError::Parse(
        "Failed to parse roast after 3 attempts".to_string(),
    ))
}

/// Parse the XML response every roast agent is required to emit.
/// Expected format:
///   <reply>
///     <mention>{DISCORD_USER_ID}</mention>
///     <roast>{text}</roast>
///     <topic>{optional}</topic>
///   </reply>
pub fn parse_roast_response(raw: &str) -> Result<RoastOutput, LlmError> {
    let mention = extract_tag(raw, "mention")
        .ok_or_else(|| {
            LlmError::Parse(
                "The response is missing the `<mention>` tag. It must contain a Discord user ID (e.g. 123456789).".to_string(),
            )
        })?;
    let roast = extract_tag(raw, "roast").ok_or_else(|| {
        LlmError::Parse(
            "The response is missing the `<roast>` tag. It must contain the roast text."
                .to_string(),
        )
    })?;
    let topic = extract_tag(raw, "topic");

    let mention = mention.trim();
    if mention.is_empty() {
        return Err(LlmError::Parse(
            "The `<mention>` tag is present but empty. It must contain a valid Discord user ID."
                .to_string(),
        ));
    }
    if !mention.chars().all(|c| c.is_ascii_digit()) {
        return Err(LlmError::Parse(
            "The `<mention>` tag must contain a valid numeric Discord user ID.".to_string(),
        ));
    }
    if roast.trim().is_empty() {
        return Err(LlmError::Parse(
            "The `<roast>` tag is present but empty. It must contain the roast text.".to_string(),
        ));
    }

    Ok(RoastOutput {
        mention_id: mention.trim().to_string(),
        roast: roast.trim().to_string(),
        topic: topic.map(|s| s.trim().to_string()),
    })
}

fn extract_tag<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text.find(&close)?;
    if end <= start {
        return None;
    }
    Some(&text[start..end])
}
