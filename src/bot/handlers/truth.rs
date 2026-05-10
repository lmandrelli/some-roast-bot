use poise::serenity_prelude::{self as serenity};
use std::sync::Arc;

use crate::bot::Error;
use crate::bot::context;

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

    let channel_ctx = context::fetch_channel_context(ctx, msg.channel_id, msg.id, 20, true).await?;

    let response =
        crate::agents::roast_truth(Arc::new(ctx.clone()), msg.channel_id, &channel_ctx).await?;

    if let Some(target_id) = super::channel::extract_mentioned_user(&response) {
        crate::memory::record_roast(&msg.author.id.to_string(), Some(&target_id), "truth");
    } else {
        crate::memory::record_roast(&msg.author.id.to_string(), None, "truth");
    }

    Ok(response)
}
