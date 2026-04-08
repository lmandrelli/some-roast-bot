use poise::serenity_prelude as serenity;

use crate::bot::Error;

pub fn contains_quoi(content: &str) -> bool {
    let lower = content.to_lowercase();
    let trimmed = lower.trim_end_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace());
    trimmed.ends_with("quoi") && !trimmed.ends_with("pourquoi")
}

pub async fn handle_quoi(
    _ctx: &serenity::Context,
    _msg: &serenity::Message,
) -> Result<String, Error> {
    Ok("-feur".to_string())
}
