//! The user message sent to the model: instruction, Discord context and
//! images, in one call.

use std::collections::{HashMap, HashSet};

use rig::{
    OneOrMany,
    message::{Image, Message, UserContent},
};

use crate::bot::context::{ChannelContext, canonical_image_key, formatter};

/// One user message with the whole context: the instruction, the transcript,
/// and each downloaded image right after the message that posted it.
pub fn prompt_message(
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
    for (message, text) in context
        .messages
        .iter()
        .zip(formatter::format_messages(context))
    {
        content.push(UserContent::text(text));
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
            reply_quote: None,
            forwarded: vec![],
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
            guild_name: "g".into(),
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
        assert!(parts[1].starts_with("#1 ·"));
        assert_eq!(parts[2], "image:cat");
        assert!(parts[3].starts_with("#2 ·"));
        assert!(parts[4].starts_with("#3 ·"));
    }

    #[test]
    fn trigger_quotes_the_message_it_replies_to() {
        let mut target = transcript_message(1, &[]);
        target.is_reply_target = true;
        let mut trigger = transcript_message(2, &[]);
        trigger.is_trigger = true;
        trigger.content = "@Some(roast) .".into();
        trigger.reply_to = Some((target.id, Some(target.author.clone())));
        let context = ChannelContext {
            guild_name: "g".into(),
            channel_name: "c".into(),
            channel_description: None,
            messages: vec![target, trigger],
        };
        let blocks = formatter::format_messages(&context);
        assert!(blocks[0].contains("[target of the trigger]"));
        assert!(blocks[1].starts_with("── [TRIGGER]"));
        assert!(blocks[1].contains("↳ replies to #1 (u1): « message 1 »"));
    }
}
