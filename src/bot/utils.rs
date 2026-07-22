use poise::serenity_prelude as serenity;
use std::collections::HashSet;

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
    let user_re = regex::Regex::new(r"<@!?(\d+)>").unwrap();
    let mut allowed = HashSet::new();
    let mut valid = HashSet::new();
    let mut attempted = HashSet::new();
    if let Some(guild_id) = guild_id {
        for captures in user_re.captures_iter(content) {
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
    let text = user_re.replace_all(content, |caps: &regex::Captures<'_>| {
        let id = caps[1].parse::<u64>().ok().map(serenity::UserId::new);
        if let Some(id) = id.filter(|id| valid.contains(id)) {
            allowed.insert(id);
            format!("<@{id}>")
        } else {
            "<filtered>".into()
        }
    });
    let roles = regex::Regex::new(r"<@&\d+>").unwrap();
    let text = roles
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

pub async fn send_roast(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    text: &str,
    allowed_users: &[serenity::UserId],
) -> Result<(), serenity::Error> {
    const LIMIT: usize = 2000;
    let mut remaining = text;
    while !remaining.is_empty() {
        let end = if remaining.len() <= LIMIT {
            remaining.len()
        } else {
            let mut end = LIMIT;
            while !remaining.is_char_boundary(end) {
                end -= 1;
            }
            remaining[..end].rfind(char::is_whitespace).unwrap_or(end)
        };
        let chunk = &remaining[..end];
        let mentions = serenity::CreateAllowedMentions::new()
            .all_users(false)
            .all_roles(false)
            .everyone(false)
            .users(allowed_users.to_vec())
            .replied_user(false);
        channel_id
            .send_message(
                &ctx.http,
                serenity::CreateMessage::new()
                    .content(chunk)
                    .allowed_mentions(mentions),
            )
            .await?;
        remaining = remaining[end..].trim_start();
    }
    Ok(())
}
