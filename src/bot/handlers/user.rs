use async_trait::async_trait;
use poise::serenity_prelude::Mentionable;

use crate::bot::context;
use crate::bot::handler::{HandlerContext, MessageHandler};
use crate::error::BotError;

/// Handler for targeted user roasts (bot tagged alongside another user).
pub struct UserHandler;

#[async_trait]
impl MessageHandler for UserHandler {
    fn name(&self) -> &'static str {
        "user"
    }

    fn priority(&self) -> u8 {
        11
    }

    async fn can_handle(&self, ctx: &HandlerContext<'_>) -> bool {
        if !ctx.mentions_me {
            return false;
        }

        let other_mentions: Vec<_> = ctx
            .message
            .mentions
            .iter()
            .filter(|u| u.id != ctx.bot_id && !u.bot)
            .collect();

        !other_mentions.is_empty()
    }

    async fn handle(&self, ctx: &HandlerContext<'_>) -> Result<Option<String>, BotError> {
        let msg = ctx.message;
        let target_user = msg
            .mentions
            .iter()
            .filter(|u| u.id != ctx.bot_id && !u.bot)
            .next()
            .unwrap(); // safe: can_handle validated this

        tracing::info!(
            "User roast - {} wants to roast {}",
            msg.author.name,
            target_user.name
        );

        let channel_ctx =
            context::fetch_channel_context(ctx.serenity_ctx, msg.channel_id, msg, true).await?;

        let tagger_name = &msg.author.name;
        let target_name = &target_user.name;
        let target_mention = target_user.id.mention().to_string();

        let output = crate::agents::roast_user(
            &ctx.llm_service,
            tagger_name,
            target_name,
            &target_mention,
            &channel_ctx,
        )
        .await?;

        ctx.memory
            .record_roast(&msg.author.id.to_string(), Some(&output.mention_id), "user")
            .map_err(|e| BotError::Db(e))?;

        Ok(Some(output.roast))
    }
}
