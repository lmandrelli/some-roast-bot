use poise::serenity_prelude::{self as serenity, Mentionable};

use crate::bot::Error;

/// Checks whether a message contains "is this true?" or "is that true?"
/// (case-insensitive, tolerant of an optional space before the question mark).
pub fn contains_truth_question(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("is this true?")
        || lower.contains("is this true ?")
        || lower.contains("is that true?")
        || lower.contains("is that true ?")
}

/// Responds to "is this true?" by fetching recent channel messages
/// and letting the model judge the claim.
pub async fn handle_truth(
    ctx: &serenity::Context,
    msg: &serenity::Message,
) -> Result<String, Error> {
    tracing::info!(
        "Truth check triggered by {} in channel {}",
        msg.author.name,
        msg.channel_id
    );

    let builder = serenity::builder::GetMessages::new()
        .before(msg.id)
        .limit(10);
    let messages = msg.channel_id.messages(&ctx.http, builder).await?;

    let context_messages: Vec<(String, String, String)> = messages
        .iter()
        .filter(|m| !m.author.bot)
        .map(|m| {
            (
                m.author.name.clone(),
                m.author.id.mention().to_string(),
                m.content.clone(),
            )
        })
        .collect();

    crate::agents::roast_truth(&context_messages).await
}
