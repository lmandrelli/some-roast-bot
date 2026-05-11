use crate::bot::Error;
use crate::fixers;
use poise::serenity_prelude as serenity;

/// Detect social-media links in a message, delete the original message,
/// and post the fixed (embed-friendly) URLs as two separate messages.
///
/// Returns `true` when at least one link was handled.
pub async fn handle_social_links(
    ctx: &serenity::Context,
    msg: &serenity::Message,
) -> Result<bool, Error> {
    let fixed = fixers::fix_links(&msg.content).await;
    if fixed.is_empty() {
        return Ok(false);
    }

    // Delete the user's original message.
    if let Err(e) = msg.delete(&ctx.http).await {
        tracing::warn!("Failed to delete original msg {}: {}", msg.id, e);
    }

    let urls = fixed
        .iter()
        .map(|l| l.fixed_url.clone())
        .collect::<Vec<_>>()
        .join(" ");

    let header_text = format!("{} posted :", msg.author.name);

    msg.channel_id.say(&ctx.http, header_text).await?;
    msg.channel_id.say(&ctx.http, urls).await?;

    // Stats
    crate::memory::record_roast(&msg.author.id.to_string(), None, "social_link");

    Ok(true)
}
