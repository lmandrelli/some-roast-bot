use poise::serenity_prelude as serenity;
use std::collections::HashSet;
use std::sync::LazyLock;

static USER_MENTION_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"<@!?(\d+)>").unwrap());
static ROLE_MENTION_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"<@&\d+>").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeResponse {
    pub text: String,
    pub allowed_users: Vec<serenity::UserId>,
}

pub async fn sanitize_mentions(
    ctx: &serenity::Context,
    guild_id: Option<serenity::GuildId>,
    self_id: serenity::UserId,
    content: &str,
) -> SafeResponse {
    let mut allowed = HashSet::new();
    let mut valid = HashSet::new();
    let mut attempted = HashSet::new();
    if let Some(guild_id) = guild_id {
        for captures in USER_MENTION_RE.captures_iter(content) {
            if let Ok(id) = captures[1].parse::<u64>() {
                let id = serenity::UserId::new(id);
                if !attempted.insert(id) || id == self_id {
                    continue;
                }
                if id != self_id
                    && (ctx
                        .cache
                        .guild(guild_id)
                        .is_some_and(|g| g.members.contains_key(&id))
                        || guild_id.member(&ctx.http, id).await.is_ok())
                {
                    valid.insert(id);
                }
            }
        }
    }
    let text = USER_MENTION_RE.replace_all(content, |caps: &regex::Captures<'_>| {
        let id = caps[1].parse::<u64>().ok().map(serenity::UserId::new);
        if let Some(id) = id.filter(|id| valid.contains(id)) {
            allowed.insert(id);
            format!("<@{id}>")
        } else {
            "<filtered>".into()
        }
    });
    let text = ROLE_MENTION_RE
        .replace_all(&text, "<filtered>")
        .replace("@everyone", "@\u{200b}everyone")
        .replace("@here", "@\u{200b}here");
    let mut allowed_users = allowed.into_iter().collect::<Vec<_>>();
    allowed_users.sort_by_key(|id| id.get());
    SafeResponse {
        text,
        allowed_users,
    }
}

/// Discord message length limit (counted in bytes here, which is stricter).
const MESSAGE_LIMIT: usize = 2000;

/// Split `text` into Discord-sized chunks, on whitespace when possible.
pub fn split_message(text: &str) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut remaining = text.trim();
    while !remaining.is_empty() {
        let end = if remaining.len() <= MESSAGE_LIMIT {
            remaining.len()
        } else {
            let mut end = MESSAGE_LIMIT;
            while !remaining.is_char_boundary(end) {
                end -= 1;
            }
            remaining[..end]
                .rfind(char::is_whitespace)
                .filter(|&i| i > 0)
                .unwrap_or(end)
        };
        chunks.push(&remaining[..end]);
        remaining = remaining[end..].trim_start();
    }
    chunks
}

/// Only the sanitized users may be pinged: no roles, no @everyone.
pub fn allowed_mentions(allowed_users: &[serenity::UserId]) -> serenity::CreateAllowedMentions {
    serenity::CreateAllowedMentions::new()
        .all_users(false)
        .all_roles(false)
        .everyone(false)
        .users(allowed_users.to_vec())
        .replied_user(false)
}

pub async fn send_roast(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    text: &str,
    allowed_users: &[serenity::UserId],
) -> Result<(), crate::error::BotError> {
    for chunk in split_message(text) {
        channel_id
            .send_message(
                &ctx.http,
                serenity::CreateMessage::new()
                    .content(chunk)
                    .allowed_mentions(allowed_mentions(allowed_users)),
            )
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_message_keeps_chunks_under_the_limit_on_whitespace() {
        assert_eq!(split_message("  court  "), vec!["court"]);
        assert!(split_message("   ").is_empty());
        let long = format!("{} {}", "é".repeat(900), "a".repeat(400));
        let chunks = split_message(&long);
        assert_eq!(
            chunks,
            vec!["é".repeat(900).as_str(), "a".repeat(400).as_str()]
        );
        let unbroken = "x".repeat(4500);
        let chunks = split_message(&unbroken);
        assert_eq!(
            chunks.iter().map(|c| c.len()).collect::<Vec<_>>(),
            vec![2000, 2000, 500]
        );
    }
}
