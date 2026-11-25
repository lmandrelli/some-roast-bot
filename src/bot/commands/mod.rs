mod ask;
mod fix;
mod research;
mod stats;

pub use ask::ask;
pub use fix::fix;
pub use research::research;
pub use stats::stats;

use crate::bot::context::{self, AuthorIdentity, ChannelContext};
use crate::bot::{Context, Error, utils};

/// The asker and the recent channel messages. Without access to the
/// channel history, the question is answered without context.
async fn question_context(ctx: Context<'_>) -> (AuthorIdentity, ChannelContext) {
    let author = ctx.author();
    let name = match ctx.author_member().await {
        Some(member) => member.display_name().to_string(),
        None => author.display_name().to_string(),
    };
    let asker = AuthorIdentity {
        id: author.id,
        name,
        is_bot: author.bot,
        is_self: false,
    };
    let channel = match context::fetch_recent_context(
        ctx.serenity_context(),
        ctx.channel_id(),
        ctx.guild_id(),
        context::SLASH_CONTEXT_LIMIT,
    )
    .await
    {
        Ok(channel) => channel,
        Err(error) => {
            tracing::warn!("Channel context unavailable: {error}");
            ChannelContext {
                guild_name: ctx
                    .guild()
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|| "Direct message".into()),
                channel_name: "unknown".into(),
                channel_description: None,
                messages: vec![],
            }
        }
    };
    (asker, channel)
}

/// Send a model answer: mentions filtered like roasts, split at the
/// Discord length limit.
async fn send_answer(ctx: Context<'_>, text: &str) -> Result<(), Error> {
    let bot_id = ctx.serenity_context().cache.current_user().id;
    let safe = utils::sanitize_mentions(ctx.serenity_context(), ctx.guild_id(), bot_id, text).await;
    for chunk in utils::split_message(&safe.text) {
        ctx.send(
            poise::CreateReply::default()
                .content(chunk)
                .allowed_mentions(utils::allowed_mentions(&safe.allowed_users)),
        )
        .await?;
    }
    Ok(())
}
