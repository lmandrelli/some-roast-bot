use poise::serenity_prelude::{self as serenity};
use std::sync::Arc;

use crate::bot::Error;
use crate::bot::context;

/// Priority 3: Bot tagged alone.
/// Picks someone from recent channel messages and roasts them.
pub async fn handle_channel(
    ctx: &serenity::Context,
    msg: &serenity::Message,
) -> Result<String, Error> {
    tracing::info!(
        "Priority 3: Channel roast triggered by {}",
        msg.author.name
    );

    let channel_ctx = context::fetch_channel_context(
        ctx,
        msg.channel_id,
        msg.id,
        20,
        true,
    ).await?;

    crate::agents::roast_channel(Arc::new(ctx.clone()), msg.channel_id, &channel_ctx).await
}
