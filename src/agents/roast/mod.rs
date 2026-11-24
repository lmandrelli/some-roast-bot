mod channel;
mod microsoft;
mod truth;
pub use channel::roast_channel;
pub use microsoft::roast_microsoft;
pub use truth::roast_truth;

use crate::{
    agents::llm::LlmService,
    bot::context::{ChannelContext, canonical_image_key, formatter},
    error::LlmError,
};
use rig::{
    OneOrMany,
    completion::Prompt,
    message::{Image, Message, UserContent},
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RoastOutput {
    pub response: String,
    #[serde(default)]
    pub topic: Option<String>,
}

/// One user message with the whole context: the instruction, the transcript,
/// and each downloaded image right after the message that posted it.
fn prompt_message(
    text: String,
    context: &ChannelContext,
    images: &HashMap<String, Image>,
) -> Message {
    let mut content = vec![UserContent::text(format!(
        "{}\n{}",
        text,
        formatter::format_header(context)
    ))];
    let mut attached = HashSet::new();
    for message in &context.messages {
        content.push(UserContent::text(formatter::format_transcript_message(
            message,
        )));
        for visual in &message.visuals {
            let key = canonical_image_key(&visual.url);
            if let Some(image) = images.get(&key)
                && attached.insert(key)
            {
                content.push(UserContent::Image(image.clone()));
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
    use crate::bot::context::{AuthorIdentity, TranscriptMessage, Visual};
    use poise::serenity_prelude as serenity;
    use rig::message::DocumentSourceKind;

    fn transcript_message(id: u64, urls: &[&str]) -> TranscriptMessage {
        TranscriptMessage {
            id: serenity::MessageId::new(id),
            timestamp: serenity::Timestamp::from_unix_timestamp(id as i64).unwrap(),
            author: AuthorIdentity {
                id: serenity::UserId::new(id),
                name: format!("u{id}"),
                is_bot: false,
                is_self: false,
            },
            content: format!("message {id}"),
            is_trigger: false,
            is_reply_target: false,
            reply_to: None,
            attachments: vec![],
            stickers: vec![],
            embeds: vec![],
            visuals: urls
                .iter()
                .map(|url| Visual {
                    url: (*url).into(),
                    mime: None,
                })
                .collect(),
        }
    }

    fn image(data: &str) -> Image {
        Image {
            data: DocumentSourceKind::Base64(data.into()),
            media_type: None,
            detail: None,
            additional_params: None,
        }
    }

    /// Text parts as their first line, images as `image:<data>`.
    fn parts(message: Message) -> Vec<String> {
        let Message::User { content } = message else {
            panic!("expected a user message");
        };
        content
            .into_iter()
            .map(|part| match part {
                UserContent::Text(text) => text.text.lines().next().unwrap_or("").to_string(),
                UserContent::Image(Image {
                    data: DocumentSourceKind::Base64(data),
                    ..
                }) => format!("image:{data}"),
                _ => panic!("unexpected content"),
            })
            .collect()
    }

    #[test]
    fn images_follow_the_message_that_posted_them_once() {
        let cat = "https://cdn.discordapp.com/attachments/1/cat.png?ex=a";
        let cat_resigned = "https://cdn.discordapp.com/attachments/1/cat.png?ex=b";
        let missing = "https://example.com/not-downloaded.png";
        let context = ChannelContext {
            guild_id: None,
            guild_name: "g".into(),
            channel_id: serenity::ChannelId::new(1),
            channel_name: "c".into(),
            channel_description: None,
            messages: vec![
                transcript_message(1, &[cat, missing]),
                transcript_message(2, &[]),
                transcript_message(3, &[cat_resigned]),
            ],
        };
        let images = HashMap::from([(canonical_image_key(cat), image("cat"))]);
        let parts = parts(prompt_message("roast".into(), &context, &images));
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0], "roast");
        assert!(parts[1].contains("message_id=1"));
        assert_eq!(parts[2], "image:cat");
        assert!(parts[3].contains("message_id=2"));
        assert!(parts[4].contains("message_id=3"));
    }
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
