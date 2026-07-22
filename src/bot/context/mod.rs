use std::collections::{HashMap, HashSet};

use poise::serenity_prelude as serenity;

use crate::error::BotError;

pub mod formatter;

const CONTEXT_FETCH_LIMIT: usize = 14;
pub const VISUAL_LIMIT: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorIdentity {
    pub id: serenity::UserId,
    pub name: String,
    pub is_bot: bool,
    pub is_self: bool,
}

#[derive(Debug, Clone)]
pub struct AttachmentData {
    pub filename: String,
    pub description: Option<String>,
    pub content_type: Option<String>,
    pub size: u32,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct StickerData {
    pub name: String,
    pub format: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EmbedData {
    pub title: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub provider: Option<String>,
    pub fields: Vec<(String, String, bool)>,
    pub footer: Option<String>,
    pub url: Option<String>,
    pub image: Option<Visual>,
    pub thumbnail: Option<Visual>,
    pub video: Option<(String, Option<u32>, Option<u32>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visual {
    pub url: String,
    pub mime: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TranscriptMessage {
    pub id: serenity::MessageId,
    pub timestamp: serenity::Timestamp,
    pub author: AuthorIdentity,
    pub content: String,
    pub is_trigger: bool,
    pub is_reply_target: bool,
    pub reply_to: Option<(serenity::MessageId, Option<AuthorIdentity>)>,
    pub attachments: Vec<AttachmentData>,
    pub stickers: Vec<StickerData>,
    pub embeds: Vec<EmbedData>,
    pub visuals: Vec<Visual>,
}

#[derive(Debug, Clone)]
pub struct ChannelContext {
    pub guild_id: Option<serenity::GuildId>,
    pub guild_name: String,
    pub channel_id: serenity::ChannelId,
    pub channel_name: String,
    pub channel_description: Option<String>,
    pub messages: Vec<TranscriptMessage>,
}

impl ChannelContext {
    pub fn trigger_content(&self) -> Option<&str> {
        self.messages
            .iter()
            .find(|m| m.is_trigger)
            .map(|m| m.content.as_str())
    }
}

impl std::fmt::Display for ChannelContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&formatter::format_channel_context(self))
    }
}

pub async fn fetch_channel_context(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    trigger: &serenity::Message,
) -> Result<ChannelContext, BotError> {
    let mut raw = channel_id
        .messages(
            &ctx.http,
            serenity::builder::GetMessages::new()
                .before(trigger.id)
                .limit(CONTEXT_FETCH_LIMIT as u8),
        )
        .await?;
    raw.push(trigger.clone());
    if let Some(replied) = trigger.referenced_message.as_deref() {
        if !raw.iter().any(|m| m.id == replied.id) {
            raw.push(replied.clone());
        }
    } else if let Some(reference) = &trigger.message_reference {
        if let Some(id) = reference.message_id {
            if !raw.iter().any(|m| m.id == id) {
                if let Ok(message) = channel_id.message(&ctx.http, id).await {
                    raw.push(message);
                }
            }
        }
    }
    raw.sort_by_key(|m| (m.timestamp.unix_timestamp(), m.id.get()));
    raw.dedup_by_key(|m| m.id);
    // A malformed/future timestamp must never place anything after the trigger.
    if let Some(pos) = raw.iter().position(|m| m.id == trigger.id) {
        let item = raw.remove(pos);
        raw.push(item);
    }

    let guild_id = trigger.guild_id;
    let self_id = ctx.cache.current_user().id;
    let mut identities = HashMap::new();
    for message in &raw {
        let user = &message.author;
        if identities.contains_key(&user.id) {
            continue;
        }
        let mut name = message.member.as_ref().and_then(|m| m.nick.clone());
        if name.is_none() {
            name = guild_id.and_then(|gid| {
                ctx.cache
                    .guild(gid)
                    .and_then(|g| g.members.get(&user.id).and_then(|m| m.nick.clone()))
            });
        }
        if name.is_none() {
            if let Some(gid) = guild_id {
                name = gid
                    .member(&ctx.http, user.id)
                    .await
                    .ok()
                    .and_then(|m| m.nick);
            }
        }
        identities.insert(
            user.id,
            AuthorIdentity {
                id: user.id,
                name: name.unwrap_or_else(|| {
                    user.global_name
                        .clone()
                        .unwrap_or_else(|| user.name.clone())
                }),
                is_bot: user.bot,
                is_self: user.id == self_id,
            },
        );
    }

    let reply_target_id = trigger
        .referenced_message
        .as_ref()
        .map(|m| m.id)
        .or_else(|| {
            trigger
                .message_reference
                .as_ref()
                .and_then(|r| r.message_id)
        });
    let mut messages = Vec::new();
    for message in &raw {
        let content = expand_mentions(&message.content, &message.mentions, &identities);
        let attachments = message
            .attachments
            .iter()
            .map(|a| AttachmentData {
                filename: a.filename.clone(),
                description: a.description.clone(),
                content_type: a.content_type.clone(),
                size: a.size,
                width: a.width,
                height: a.height,
                url: a.url.clone(),
            })
            .collect::<Vec<_>>();
        let stickers = message
            .sticker_items
            .iter()
            .map(|s| StickerData {
                name: s.name.clone(),
                format: format!("{:?}", s.format_type),
                url: s.image_url(),
            })
            .collect::<Vec<_>>();
        let embeds = message.embeds.iter().map(embed_data).collect::<Vec<_>>();
        let mut visuals = Vec::new();
        for a in &attachments {
            if a.content_type
                .as_deref()
                .is_some_and(|m| matches!(m, "image/png" | "image/jpeg" | "image/webp"))
            {
                visuals.push(Visual {
                    url: a.url.clone(),
                    mime: a.content_type.clone(),
                });
            }
            if a.content_type.as_deref() == Some("image/gif") {
                visuals.push(Visual {
                    url: a.url.clone(),
                    mime: a.content_type.clone(),
                });
            }
        }
        for s in &stickers {
            if !s.format.to_lowercase().contains("lottie") {
                if let Some(url) = &s.url {
                    visuals.push(Visual {
                        url: url.clone(),
                        mime: None,
                    });
                }
            }
        }
        for e in &embeds {
            // Discord's static thumbnail is preferable to an animated embed image.
            visuals.extend(e.thumbnail.clone());
            visuals.extend(e.image.clone());
        }
        let reply_id = message
            .message_reference
            .as_ref()
            .and_then(|r| r.message_id)
            .or_else(|| message.referenced_message.as_ref().map(|m| m.id));
        let reply_author = message
            .referenced_message
            .as_ref()
            .and_then(|m| identities.get(&m.author.id).cloned());
        messages.push(TranscriptMessage {
            id: message.id,
            timestamp: message.timestamp,
            author: identities[&message.author.id].clone(),
            content,
            is_trigger: message.id == trigger.id,
            is_reply_target: Some(message.id) == reply_target_id,
            reply_to: reply_id.map(|id| (id, reply_author)),
            attachments,
            stickers,
            embeds,
            visuals,
        });
    }
    let guild_name = guild_id
        .and_then(|id| ctx.cache.guild(id).map(|g| g.name.clone()))
        .unwrap_or_else(|| "Direct message".into());
    let channel_details = guild_id.and_then(|guild_id| {
        ctx.cache.guild(guild_id).and_then(|guild| {
            guild
                .channels
                .get(&channel_id)
                .map(|channel| (channel.name.clone(), channel.topic.clone()))
        })
    });
    let (channel_name, channel_description) = match channel_details {
        Some(details) => details,
        None => match channel_id.to_channel(&ctx.http).await {
            Ok(serenity::Channel::Guild(channel)) => (channel.name, channel.topic),
            Ok(serenity::Channel::Private(channel)) => (channel.name(), None),
            Ok(_) => ("unknown".into(), None),
            Err(_) => ("unknown".into(), None),
        },
    };
    Ok(ChannelContext {
        guild_id,
        guild_name,
        channel_id,
        channel_name,
        channel_description,
        messages,
    })
}

fn embed_data(e: &serenity::Embed) -> EmbedData {
    let visual = |url: &str, _w, _h| Visual {
        url: url.to_string(),
        mime: if url.to_lowercase().contains(".gif") {
            Some("image/gif".into())
        } else {
            None
        },
    };
    EmbedData {
        title: e.title.clone(),
        description: e.description.clone(),
        author: e.author.as_ref().map(|x| x.name.clone()),
        provider: e.provider.as_ref().and_then(|x| x.name.clone()),
        fields: e
            .fields
            .iter()
            .map(|x| (x.name.clone(), x.value.clone(), x.inline))
            .collect(),
        footer: e.footer.as_ref().map(|x| x.text.clone()),
        url: e.url.clone(),
        image: e.image.as_ref().map(|x| visual(&x.url, x.width, x.height)),
        thumbnail: e
            .thumbnail
            .as_ref()
            .map(|x| visual(&x.url, x.width, x.height)),
        video: e.video.as_ref().map(|x| (x.url.clone(), x.width, x.height)),
    }
}

pub fn expand_mentions(
    content: &str,
    mentions: &[serenity::User],
    identities: &HashMap<serenity::UserId, AuthorIdentity>,
) -> String {
    let mut result = content.to_string();
    for user in mentions {
        let name = identities
            .get(&user.id)
            .map(|i| i.name.as_str())
            .unwrap_or_else(|| user.display_name());
        let replacement = format!("@{name} (<@{}>)", user.id);
        result = result
            .replace(&format!("<@{}>", user.id), &replacement)
            .replace(&format!("<@!{}>", user.id), &replacement);
    }
    result
}

pub fn prioritize_visuals(messages: &[TranscriptMessage]) -> Vec<Visual> {
    let mut ranked = messages
        .iter()
        .filter(|m| m.is_trigger)
        .chain(
            messages
                .iter()
                .filter(|m| m.is_reply_target && !m.is_trigger),
        )
        .chain(
            messages
                .iter()
                .rev()
                .filter(|m| !m.is_trigger && !m.is_reply_target),
        );
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for message in &mut ranked {
        for visual in &message.visuals {
            if seen.insert(visual.url.clone()) {
                out.push(visual.clone());
                if out.len() == VISUAL_LIMIT {
                    return out;
                }
            }
        }
    }
    out
}

pub fn canonical_image_key(source_url: &str) -> String {
    let Ok(mut url) = url::Url::parse(source_url) else {
        return source_url.to_owned();
    };
    let is_discord_cdn = matches!(
        url.host_str(),
        Some("cdn.discordapp.com" | "media.discordapp.net")
    );
    if is_discord_cdn {
        url.set_query(None);
        url.set_fragment(None);
        url.to_string()
    } else {
        source_url.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(id: u64, trigger: bool, reply: bool, urls: &[&str]) -> TranscriptMessage {
        TranscriptMessage {
            id: serenity::MessageId::new(id),
            timestamp: serenity::Timestamp::from_unix_timestamp(id as i64).unwrap(),
            author: AuthorIdentity {
                id: serenity::UserId::new(id),
                name: format!("u{id}"),
                is_bot: false,
                is_self: false,
            },
            content: String::new(),
            is_trigger: trigger,
            is_reply_target: reply,
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

    #[test]
    fn visual_priority_deduplicates_and_caps_at_ten() {
        let old = (0..12).map(|i| format!("old-{i}")).collect::<Vec<_>>();
        let refs = old.iter().map(String::as_str).collect::<Vec<_>>();
        let messages = vec![
            message(1, false, false, &refs),
            message(2, false, true, &["reply", "same"]),
            message(3, true, false, &["trigger", "same"]),
        ];
        let result = prioritize_visuals(&messages);
        assert_eq!(result.len(), 10);
        assert_eq!(result[0].url, "trigger");
        assert_eq!(result[1].url, "same");
        assert_eq!(result[2].url, "reply");
    }

    #[test]
    fn canonicalizes_only_discord_cdn_urls() {
        assert_eq!(
            canonical_image_key("https://cdn.discordapp.com/attachments/1/a.png?ex=1&is=2#x"),
            "https://cdn.discordapp.com/attachments/1/a.png"
        );
        let external = "https://example.com/a.png?token=1#frame";
        assert_eq!(canonical_image_key(external), external);
    }

    #[test]
    fn transcript_places_description_on_owner_and_omits_visual_url() {
        let url = "https://cdn.discordapp.com/attachments/1/cat.png?ex=signed";
        let mut owner = message(1, false, false, &[url]);
        owner.attachments.push(AttachmentData {
            filename: "cat.png".into(),
            description: None,
            content_type: Some("image/png".into()),
            size: 42,
            width: Some(10),
            height: Some(10),
            url: url.into(),
        });
        let descriptions =
            HashMap::from([(canonical_image_key(url), "Un chat devant un écran".into())]);
        let formatted =
            formatter::format_transcript_message_with_descriptions(&owner, &descriptions);
        assert!(formatted.contains("visual_description: Un chat devant un écran"));
        assert!(!formatted.contains(url));

        let other = formatter::format_transcript_message_with_descriptions(
            &message(2, false, false, &[]),
            &descriptions,
        );
        assert!(!other.contains("Un chat devant un écran"));
    }
}
