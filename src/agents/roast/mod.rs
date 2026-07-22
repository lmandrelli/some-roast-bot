mod channel;
mod microsoft;
mod truth;
pub use channel::roast_channel;
pub use microsoft::roast_microsoft;
pub use truth::roast_truth;

use crate::{agents::llm::LlmService, bot::context::ChannelContext, error::LlmError};
use rig::{
    OneOrMany,
    completion::Prompt,
    message::{DocumentSourceKind, Image, Message, UserContent},
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RoastOutput {
    pub response: String,
    #[serde(default)]
    pub topic: Option<String>,
}

fn prompt_message(text: String, context: &ChannelContext, images: bool) -> Message {
    let selected = crate::bot::context::prioritize_visuals(&context.messages);
    let prepared = selected
        .iter()
        .map(|v| v.url.as_str())
        .zip(context.visuals.iter())
        .collect::<HashMap<_, _>>();
    let mut content = vec![UserContent::text(format!(
        "{}\n{}",
        text,
        crate::bot::context::formatter::format_header(context)
    ))];
    let mut emitted = HashSet::new();
    for message in &context.messages {
        content.push(UserContent::text(
            crate::bot::context::formatter::format_transcript_message(message),
        ));
        if images {
            for visual in &message.visuals {
                if emitted.insert(visual.url.as_str())
                    && let Some(prepared) = prepared.get(visual.url.as_str())
                {
                    content.push(UserContent::Image(Image {
                        data: DocumentSourceKind::Url(prepared.url.clone()),
                        ..Default::default()
                    }));
                }
            }
        }
    }
    Message::User {
        content: OneOrMany::many(content).expect("prompt always has text"),
    }
}

pub async fn try_roast_with_retry(
    llm: &LlmService,
    preamble: &str,
    prompt: &str,
    context: &ChannelContext,
) -> Result<RoastOutput, LlmError> {
    let (agent, _session) = llm.build_agent(preamble).await?;
    let mut previous = String::new();
    let mut error = String::new();
    let mut images = !context.visuals.is_empty();
    for attempt in 0..3 {
        let text = if error.is_empty() {
            prompt.to_string()
        } else {
            format!(
                "{prompt}\n\nYour previous output was invalid:\n```\n{previous}\n```\nValidation error: {error}\nReturn ONLY corrected JSON with a non-empty response string and optional topic string."
            )
        };
        let response = match agent
            .prompt(prompt_message(text.clone(), context, images))
            .max_turns(4)
            .await
        {
            Ok(value) => value,
            Err(e) if images => {
                tracing::warn!("multimodal request failed; retrying once without images: {e}");
                images = false;
                agent
                    .prompt(prompt_message(text, context, false))
                    .max_turns(4)
                    .await
                    .map_err(|e| LlmError::Completion(e.to_string()))?
            }
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
