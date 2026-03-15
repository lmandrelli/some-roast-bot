use poise::serenity_prelude::{self as serenity, Mentionable};

use crate::bot::Error;

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

    crate::agents::roast_channel(&context_messages).await
}
