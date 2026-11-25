mod channel;
mod microsoft;
mod truth;
pub use channel::roast_channel;
pub use microsoft::roast_microsoft;
pub use truth::roast_truth;

use crate::{
    agents::{llm::LlmService, prompt::prompt_message},
    bot::context::ChannelContext,
    error::LlmError,
};
use rig::completion::Prompt;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RoastOutput {
    pub response: String,
    #[serde(default)]
    pub topic: Option<String>,
}

/// Points the model at the message the trigger replies to, when there is one.
pub fn reply_hint(context: &ChannelContext) -> Option<String> {
    let trigger = context.messages.iter().find(|m| m.is_trigger)?;
    let (id, author) = trigger.reply_to.as_ref()?;
    let author = author
        .as_ref()
        .map(|a| a.name.as_str())
        .unwrap_or("someone");
    Some(match context.messages.iter().position(|m| m.id == *id) {
        Some(index) if crate::bot::context::is_bare(&trigger.content) => format!(
            "The [TRIGGER] says nothing besides your mention: it points at message #{} by {author}. Roast {author} on that message, including what its link, image or forwarded content shows.",
            index + 1
        ),
        Some(index) => format!(
            "The [TRIGGER] replies to message #{} by {author}: that is your target, unless the trigger clearly names another one.",
            index + 1
        ),
        None if trigger.reply_quote.is_some() => format!(
            "The [TRIGGER] replies to an older message by {author}, quoted on its ↳ line: that is your target, unless the trigger clearly names another one."
        ),
        None => "The [TRIGGER] replies to a deleted message: work out the target from the recent conversation.".to_string(),
    })
}

pub async fn try_roast_with_retry(
    llm: &LlmService,
    preamble: &str,
    prompt: &str,
    context: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let mut context = context.clone();
    crate::agents::links::add_link_previews(&mut context).await;
    let context = &context;
    let images = crate::agents::images::load_context_images(context).await;
    let (agent, _session) = llm.build_agent(preamble).await?;
    let mut previous = String::new();
    let mut error = String::new();
    for attempt in 0..3 {
        let text = if error.is_empty() {
            prompt.to_string()
        } else {
            format!(
                "{prompt}\n\nYour previous output was invalid:\n```\n{previous}\n```\nValidation error: {error}\nReturn ONLY corrected JSON with a non-empty response string and optional topic string."
            )
        };
        let response = match agent
            .prompt(prompt_message(text.clone(), context, &images))
            .max_turns(4)
            .await
        {
            Ok(value) => value,
            Err(e) => return Err(LlmError::Completion(e.to_string())),
        };
        match parse_roast_response(&response) {
            Ok(out) => return Ok(out),
            Err(e) => {
                tracing::warn!("roast JSON parse failed (attempt {}): {e}", attempt + 1);
                previous = response;
                error = e.to_string();
            }
        }
    }
    Err(LlmError::Parse(
        "Failed to parse roast JSON after 3 attempts".into(),
    ))
}

pub fn parse_roast_response(raw: &str) -> Result<RoastOutput, LlmError> {
    let trimmed = raw.trim();
    let json = if trimmed.starts_with("```json") {
        trimmed
            .trim_start_matches("```json")
            .trim_end_matches("```")
            .trim()
    } else if trimmed.starts_with("```") {
        trimmed.trim_matches('`').trim()
    } else {
        trimmed
    };
    let output: RoastOutput =
        serde_json::from_str(json).map_err(|e| LlmError::Parse(format!("invalid JSON: {e}")))?;
    if output.response.trim().is_empty() {
        return Err(LlmError::Parse(
            "response must be a non-empty string".into(),
        ));
    }
    Ok(RoastOutput {
        response: output.response.trim().into(),
        topic: output
            .topic
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_json_contract() {
        assert_eq!(
            parse_roast_response(r#"{"response":"salut","topic":"news"}"#).unwrap(),
            RoastOutput {
                response: "salut".into(),
                topic: Some("news".into())
            }
        );
    }
    #[test]
    fn rejects_missing_response() {
        assert!(parse_roast_response(r#"{"topic":"x"}"#).is_err());
    }
}
