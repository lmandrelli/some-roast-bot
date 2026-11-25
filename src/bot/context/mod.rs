use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use poise::serenity_prelude as serenity;

use crate::error::BotError;

pub mod formatter;

/// Messages fetched before a trigger.
const CONTEXT_FETCH_LIMIT: usize = 20;
/// Messages fetched for a slash command, which has no trigger message.
pub const SLASH_CONTEXT_LIMIT: u8 = 15;
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
    /// Short text of the replied message, from the reply payload: it is
    /// there even when that message is outside the fetched window.
    pub reply_quote: Option<String>,
    /// Text of the forwarded messages (Discord forwards keep the content in
    /// snapshots, the message itself is empty).
    pub forwarded: Vec<String>,
    pub attachments: Vec<AttachmentData>,
    pub stickers: Vec<StickerData>,
    pub embeds: Vec<EmbedData>,
    pub visuals: Vec<Visual>,
}

#[derive(Debug, Clone)]
pub struct ChannelContext {
    pub guild_name: String,
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
    } else if is_reply(trigger)
        && let Some(reference) = &trigger.message_reference
        && let Some(id) = reference.message_id
        && !raw.iter().any(|m| m.id == id)
        && let Ok(message) = channel_id.message(&ctx.http, id).await
    {
        raw.push(message);
    }
    raw.sort_by_key(|m| (m.timestamp.unix_timestamp(), m.id.get()));
    raw.dedup_by_key(|m| m.id);
    // A malformed/future timestamp must never place anything after the trigger.
    if let Some(pos) = raw.iter().position(|m| m.id == trigger.id) {
        let item = raw.remove(pos);
        raw.push(item);
    }

    let reply_target_id = reply_id(trigger);
    build_context(
        ctx,
        channel_id,
        trigger.guild_id,
        raw,
        Some(trigger.id),
        reply_target_id,
    )
    .await
}

/// The last `limit` messages of a channel, for a slash command.
pub async fn fetch_recent_context(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    guild_id: Option<serenity::GuildId>,
    limit: u8,
) -> Result<ChannelContext, BotError> {
    let mut raw = channel_id
        .messages(
            &ctx.http,
            serenity::builder::GetMessages::new().limit(limit),
        )
        .await?;
    raw.sort_by_key(|m| (m.timestamp.unix_timestamp(), m.id.get()));
    build_context(ctx, channel_id, guild_id, raw, None, None).await
}

async fn build_context(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    guild_id: Option<serenity::GuildId>,
    raw: Vec<serenity::Message>,
    trigger_id: Option<serenity::MessageId>,
    reply_target_id: Option<serenity::MessageId>,
) -> Result<ChannelContext, BotError> {
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
        if name.is_none()
            && let Some(gid) = guild_id
        {
            name = gid
                .member(&ctx.http, user.id)
                .await
                .ok()
                .and_then(|m| m.nick);
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

    let messages = raw
        .iter()
        .map(|m| transcript_message(m, &identities, self_id, trigger_id, reply_target_id))
        .collect();
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
        guild_name,
        channel_name,
        channel_description,
        messages,
    })
}

/// Pure conversion of one Discord message, given the resolved author names.
pub fn transcript_message(
    message: &serenity::Message,
    identities: &HashMap<serenity::UserId, AuthorIdentity>,
    self_id: serenity::UserId,
    trigger_id: Option<serenity::MessageId>,
    reply_target_id: Option<serenity::MessageId>,
) -> TranscriptMessage {
    let content = expand_mentions(&message.content, &message.mentions, identities);
    let forwarded = message
        .message_snapshots
        .iter()
        .map(|s| expand_mentions(&s.content, &s.mentions, identities))
        .collect::<Vec<_>>();
    let attachments = message
        .attachments
        .iter()
        .chain(
            message
                .message_snapshots
                .iter()
                .flat_map(|s| &s.attachments),
        )
        .map(|a| AttachmentData {
            filename: a.filename.clone(),
            description: a.description.clone(),
            content_type: a.content_type.clone(),
            url: a.url.clone(),
        })
        .collect::<Vec<_>>();
    let stickers = message
        .sticker_items
        .iter()
        .chain(
            message
                .message_snapshots
                .iter()
                .flat_map(|s| &s.sticker_items),
        )
        .map(|s| StickerData {
            name: s.name.clone(),
            format: format!("{:?}", s.format_type),
            url: s.image_url(),
        })
        .collect::<Vec<_>>();
    let embeds = message
        .embeds
        .iter()
        .chain(message.message_snapshots.iter().flat_map(|s| &s.embeds))
        .map(embed_data)
        .collect::<Vec<_>>();
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
        if !s.format.to_lowercase().contains("lottie")
            && let Some(url) = &s.url
        {
            visuals.push(Visual {
                url: url.clone(),
                mime: None,
            });
        }
    }
    for e in &embeds {
        // Discord's static thumbnail is preferable to an animated embed image.
        visuals.extend(e.thumbnail.clone());
        visuals.extend(e.image.clone());
    }
    let reply_author = message.referenced_message.as_ref().map(|m| {
        identities
            .get(&m.author.id)
            .cloned()
            .unwrap_or_else(|| AuthorIdentity {
                id: m.author.id,
                name: m
                    .author
                    .global_name
                    .clone()
                    .unwrap_or_else(|| m.author.name.clone()),
                is_bot: m.author.bot,
                is_self: m.author.id == self_id,
            })
    });
    TranscriptMessage {
        id: message.id,
        timestamp: message.timestamp,
        author: identities[&message.author.id].clone(),
        content,
        is_trigger: Some(message.id) == trigger_id,
        is_reply_target: Some(message.id) == reply_target_id,
        reply_to: reply_id(message).map(|id| (id, reply_author)),
        reply_quote: message.referenced_message.as_deref().map(quote_message),
        forwarded,
        attachments,
        stickers,
        embeds,
        visuals,
    }
}

static MENTION_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"@[^@\n]*?\(<@!?\d+>\)|<@!?\d+>").unwrap());

/// True when a message says nothing besides mentions and punctuation, like
/// "." or "@Some(roast) ." sent as a reply to point at a message.
pub fn is_bare(content: &str) -> bool {
    !MENTION_RE
        .replace_all(content, "")
        .chars()
        .any(char::is_alphanumeric)
}

/// A reply, as opposed to a forward: both use `message_reference`.
fn is_reply(message: &serenity::Message) -> bool {
    message
        .message_reference
        .as_ref()
        .is_some_and(|r| matches!(r.kind, serenity::MessageReferenceKind::Default))
}

pub(crate) fn reply_id(message: &serenity::Message) -> Option<serenity::MessageId> {
    if !is_reply(message) {
        return None;
    }
    message
        .referenced_message
        .as_ref()
        .map(|m| m.id)
        .or_else(|| {
            message
                .message_reference
                .as_ref()
                .and_then(|r| r.message_id)
        })
}

/// One line describing a replied message: text, forwarded text, link
/// previews and attachment names.
fn quote_message(message: &serenity::Message) -> String {
    let clean = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut parts = vec![clean(&message.content)];
    for snapshot in &message.message_snapshots {
        parts.push(format!("[transféré] {}", clean(&snapshot.content)));
    }
    let embeds = message
        .embeds
        .iter()
        .chain(message.message_snapshots.iter().flat_map(|s| &s.embeds));
    for e in embeds {
        let preview = [e.title.as_deref(), e.description.as_deref()]
            .into_iter()
            .flatten()
            .map(clean)
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" — ");
        if !preview.is_empty() {
            parts.push(format!("[aperçu : {preview}]"));
        }
    }
    let attachments = message.attachments.iter().chain(
        message
            .message_snapshots
            .iter()
            .flat_map(|s| &s.attachments),
    );
    for a in attachments {
        parts.push(format!("[pièce jointe : {}]", a.filename));
    }
    parts
        .into_iter()
        .filter(|p| !p.is_empty() && p != "[transféré] ")
        .collect::<Vec<_>>()
        .join(" ")
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
    fn bare_trigger_is_only_mentions_and_punctuation() {
        assert!(is_bare("@Some(roast) (<@999>) ."));
        assert!(is_bare("<@999> ?!"));
        assert!(!is_bare("@Some(roast) (<@999>) roast ce weeb"));
        assert!(!is_bare("@Some(roast) (<@999>) lui"));
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
    fn transcript_omits_visual_url() {
        let url = "https://cdn.discordapp.com/attachments/1/cat.png?ex=signed";
        let mut owner = message(1, false, false, &[url]);
        owner.attachments.push(AttachmentData {
            filename: "cat.png".into(),
            description: None,
            content_type: Some("image/png".into()),
            url: url.into(),
        });
        let context = ChannelContext {
            guild_name: "g".into(),
            channel_name: "c".into(),
            channel_description: None,
            messages: vec![owner],
        };
        let formatted = formatter::format_messages(&context).remove(0);
        assert!(formatted.contains("[pièce jointe : cat.png (image/png)]"));
        assert!(!formatted.contains(url));
    }
}
