use async_trait::async_trait;

use crate::bot::context;
use crate::bot::handler::{HandlerContext, MessageHandler};
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
            context::fetch_channel_context(ctx.serenity_ctx, msg.channel_id, msg, true).await?;

        let output = crate::agents::roast_channel(&ctx.llm_service, &channel_ctx).await?;

        ctx.memory
            .record_roast(&triggerer_id, Some(&output.mention_id), "channel")
            .map_err(|e| BotError::Db(e))?;

        Ok(Some(output.roast))
    }
}
