use poise::serenity_prelude::{self as serenity, Mentionable};

use crate::bot::Error;

use super::strip_mentions;

/// Checks whether a message mentions Microsoft or Windows (case-insensitive).
pub fn contains_microsoft_keywords(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("microsoft") || lower.contains("windows")
}

/// Roast anyone who dares mention Microsoft or Windows.
pub async fn handle_microsoft(msg: &serenity::Message) -> Result<String, Error> {
    tracing::info!(
        "Microsoft/Windows detected in message from {}",
        msg.author.name,
    );

    let clean_content = strip_mentions(&msg.content);
    crate::agents::roast_microsoft(
        &msg.author.name,
        &msg.author.id.mention().to_string(),
        &clean_content,
    )
    .await
}
