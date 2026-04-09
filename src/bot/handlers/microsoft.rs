use poise::serenity_prelude::{self as serenity, Mentionable};

use crate::bot::Error;
use crate::bot::context;

use super::strip_mentions;

/// Checks whether a message mentions Microsoft or Windows (case-insensitive).
pub fn contains_microsoft_keywords(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("microsoft") || lower.contains("windows")
}

/// Roast anyone who dares mention Microsoft or Windows.
pub async fn handle_microsoft(ctx: &serenity::Context, msg: &serenity::Message) -> Result<String, Error> {
    tracing::info!(
        "Microsoft/Windows detected in message from {}",
        msg.author.name,
    );

    let channel_ctx = context::fetch_channel_context(
        ctx,
        msg.channel_id,
        msg.id,
        5,
        true,
    ).await?;

    let clean_content = strip_mentions(&msg.content);
    crate::memory::record_roast(&msg.author.id.to_string(), Some(&msg.author.id.to_string()), "microsoft");
    crate::agents::roast_microsoft(
        &msg.author.name,
        &msg.author.id.mention().to_string(),
        &clean_content,
        &channel_ctx,
    )
    .await
}
