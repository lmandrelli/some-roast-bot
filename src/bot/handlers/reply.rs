use async_trait::async_trait;

use crate::bot::context;
use crate::bot::handler::{HandlerContext, MessageHandler};
use crate::error::BotError;

/// Handler for reply-chain roasts.
pub struct ReplyHandler;

#[async_trait]
impl MessageHandler for ReplyHandler {
    fn name(&self) -> &'static str {
        "reply"
    }

    fn priority(&self) -> u8 {
        10
    }

    async fn can_handle(&self, ctx: &HandlerContext<'_>) -> bool {
        ctx.mentions_me && ctx.message.referenced_message.is_some()
    }

    async fn handle(&self, ctx: &HandlerContext<'_>) -> Result<Option<String>, BotError> {
        let msg = ctx.message;
        let replied_msg = msg.referenced_message.as_ref().unwrap();

        tracing::info!(
            "Reply roast between {} and {}",
            msg.author.name,
            replied_msg.author.name
        );

        let channel_ctx =
            context::fetch_channel_context(ctx.serenity_ctx, msg.channel_id, msg, true).await?;

        let tagger_name = &msg.author.name;
        let target_name = &replied_msg.author.name;

        let output =
            crate::agents::roast_reply(&ctx.llm_service, tagger_name, target_name, &channel_ctx)
                .await?;

        ctx.memory
            .record_roast(
                &msg.author.id.to_string(),
                Some(&output.mention_id),
                "reply",
            )
            .map_err(|e| BotError::Db(e))?;

        Ok(Some(format!("<@{}> {}", output.mention_id, output.roast)))
    }
}
