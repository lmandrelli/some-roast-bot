use crate::bot::{Context, Error};
use crate::fixers;

#[poise::command(slash_command, prefix_command)]
pub async fn fix(
    ctx: Context<'_>,
    #[description = "Social media URL to fix"] url: String,
) -> Result<(), Error> {
    let fixed = fixers::fix_links(&url).await;

    if fixed.is_empty() {
        ctx.say("No fixable social media link found. Supported platforms: Twitter/X, Bluesky, Instagram, Reddit, TikTok.").await?;
        return Ok(());
    }

    let reply = fixed
        .iter()
        .map(|l| l.fixed_url.clone())
        .collect::<Vec<_>>()
        .join(" ");

    ctx.say(reply).await?;
    Ok(())
}
