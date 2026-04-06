use poise::serenity_prelude::{self as serenity, Mentionable};
use std::sync::Arc;

use crate::bot::Error;
use crate::bot::context;

/// Priority 2: Bot is tagged alongside another user.
/// Fetches their recent messages and roasts them.
pub async fn handle_user(
    ctx: &serenity::Context,
    msg: &serenity::Message,
    target_user: &serenity::User,
) -> Result<String, Error> {
    tracing::info!(
        "Priority 2: User roast - {} wants to roast {}",
        msg.author.name,
        target_user.name
    );

    let (target_messages, channel_ctx) = context::fetch_user_context(
        ctx,
        msg.channel_id,
        msg.id,
        target_user.id,
        25,
        5,
    ).await?;

    let tagger_name = &msg.author.name;
    let target_name = &target_user.name;
    let target_mention = target_user.id.mention().to_string();

    crate::agents::roast_user(
        Arc::new(ctx.clone()),
        msg.channel_id,
        tagger_name,
        target_name,
        &target_mention,
        &target_messages,
        &channel_ctx,
    ).await
}
