use crate::bot::{Context, Error};
use crate::fixers;

#[poise::command(slash_command, prefix_command)]
pub async fn fix(
    ctx: Context<'_>,
    #[description = "Social media URL to fix"] url: String,
) -> Result<(), Error> {
    let outcome = fixers::fix_links(&url).await;

    if !outcome.failed.is_empty() {
        let e = crate::error::BotError::LinkFix(
            outcome
                .failed
                .iter()
                .map(|l| format!("{} {}: {}", l.platform, l.original_url, l.reason))
                .collect::<Vec<_>>()
                .join(" | "),
        );
        tracing::warn!("/fix failed: {}", e);
        ctx.say(crate::error::bot_error_response(&e)).await?;
    }

    let fixed = outcome.fixed;
    if fixed.is_empty() {
        if !outcome.failed.is_empty() {
            return Ok(());
        }
        ctx.say("No fixable social media link found. Supported platforms: Twitter/X, Bluesky, Instagram, Reddit, TikTok.").await?;
        return Ok(());
    }

    let reply = fixed
        .iter()
        .map(|l| l.fixed_url.clone())
        .collect::<Vec<_>>()
        .join(" ");

    ctx.say(reply).await?;

    if fixed.iter().any(|l| l.fallback) {
        ctx.say(crate::error::link_fallback_response()).await?;
    }

    Ok(())
}
