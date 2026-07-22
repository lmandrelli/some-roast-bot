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
        if !ctx.mentions_me {
            return false;
        }

        if super::is_reply_to_social_link(ctx) {
            tracing::info!(
                "Handler '{}' ignored: reply to a transformed social link in channel {}",
                self.name(),
                ctx.message.channel_id
            );
            return false;
        }

        true
    }

    async fn handle(&self, ctx: &HandlerContext<'_>) -> Result<Option<String>, BotError> {
        let msg = ctx.message;
        tracing::info!("Channel roast triggered by {}", msg.author.name);

        let triggerer_id = msg.author.id.to_string();
        let channel_ctx =
            context::fetch_channel_context(ctx.serenity_ctx, msg.channel_id, msg).await?;

        let output = crate::agents::roast_channel(&ctx.llm_service, &channel_ctx).await?;

        ctx.memory
            .record_roast(&triggerer_id, None, "general")
            .map_err(|e| BotError::Db(e))?;

        Ok(Some(output.response))
    }
}
