use async_trait::async_trait;

use crate::bot::context;
use crate::bot::handler::{HandlerContext, MessageHandler};
use crate::bot::utils;
use crate::error::BotError;

/// Handler for channel roasts (bot tagged alone).
pub struct ChannelHandler;

#[async_trait]
impl MessageHandler for ChannelHandler {
    fn name(&self) -> &'static str {
        "channel"
    }

    fn priority(&self) -> u8 {
        12
    }

    async fn can_handle(&self, ctx: &HandlerContext<'_>) -> bool {
        ctx.mentions_me
    }

    async fn handle(&self, ctx: &HandlerContext<'_>) -> Result<Option<String>, BotError> {
        let msg = ctx.message;
        tracing::info!("Channel roast triggered by {}", msg.author.name);

        let triggerer_id = msg.author.id.to_string();
        let channel_ctx =
            context::fetch_channel_context(ctx.serenity_ctx, msg.channel_id, msg.id, 20, true)
                .await?;

        let mut response = crate::agents::roast_channel(
            &ctx.llm_service,
            ctx.serenity_ctx.clone().into(),
            msg.channel_id,
            &channel_ctx,
        )
        .await?;

        // Retry up to 3 times if the bot mentions the triggerer
        for _ in 0..3 {
            if let Some(target_id) = utils::extract_mentioned_user(&response) {
                if target_id != triggerer_id {
                    ctx.memory
                        .record_roast(&triggerer_id, Some(&target_id), "channel")
                        .map_err(|e| BotError::Db(e))?;
                    return Ok(Some(response));
                }
            }

            let retry_context = format!(
                "{}\n\nTu as mentionné la mauvaise personne. Choisis quelqu'un d'autre que <@{triggerer_id}> cette fois.",
                channel_ctx.to_string()
            );
            response = crate::agents::roast_channel_with_context(
                &ctx.llm_service,
                ctx.serenity_ctx.clone().into(),
                msg.channel_id,
                &retry_context,
            )
            .await?;
        }

        if let Some(target_id) = utils::extract_mentioned_user(&response) {
            ctx.memory
                .record_roast(&triggerer_id, Some(&target_id), "channel")
                .map_err(|e| BotError::Db(e))?;
        }

        Ok(Some(response))
    }
}
