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

/// Extract a mentioned user ID from text.
pub fn extract_mentioned_user(content: &str) -> Option<String> {
    let re = regex::Regex::new(r"<@!?(\d+)>").ok()?;
    let caps = re.captures(content)?;
    caps.get(1).map(|m| m.as_str().to_string())
}
