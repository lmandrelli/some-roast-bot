use poise::serenity_prelude as serenity;

/// Remove bot mentions from message content so the prompt is cleaner.
pub fn strip_mentions(content: &str) -> String {
    let re = regex::Regex::new(r"<@!?\d+>").unwrap();
    re.replace_all(content, "").trim().to_string()
}

/// Guardrail: replace the bot's own mention with `<filtered>` so it
/// never pings itself.
pub async fn strip_self_mentions(ctx: &serenity::Context, content: &str) -> String {
    let bot_id = match ctx.http.get_current_user().await {
        Ok(user) => user.id.to_string(),
        Err(_) => return content.to_string(),
    };
    let pattern = format!(r"<@!?{bot_id}>");
    let re = regex::Regex::new(&pattern).unwrap();
    re.replace_all(content, "<filtered>").to_string()
}

/// Send a roast message, splitting at word boundaries if it exceeds
/// Discord's 2000 character limit. Follow-up parts are sent as normal
/// channel messages.
pub async fn send_roast(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    text: &str,
) -> Result<(), serenity::Error> {
    const DISCORD_LIMIT: usize = 2000;

    let mut remaining = text;
    let mut first = true;

    while !remaining.is_empty() {
        let chunk = if remaining.len() <= DISCORD_LIMIT {
            remaining
        } else {
            // Try to find a word boundary before the limit
            match remaining[..DISCORD_LIMIT].rfind(' ') {
                Some(idx) => &remaining[..idx],
                None => &remaining[..DISCORD_LIMIT],
            }
        };

        if first {
            channel_id.say(&ctx.http, chunk).await?;
            first = false;
        } else {
            channel_id.say(&ctx.http, chunk).await?;
        }

        remaining = remaining[chunk.len()..].trim_start();
    }

    Ok(())
}
