use async_trait::async_trait;
use poise::serenity_prelude::Mentionable;

use crate::bot::context;
use crate::bot::handler::{HandlerContext, MessageHandler};
use crate::bot::utils;
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
            context::fetch_channel_context(ctx.serenity_ctx, msg.channel_id, msg.id, 10, true)
                .await?;

        ctx.memory
            .record_roast(
                &msg.author.id.to_string(),
                Some(&replied_msg.author.id.to_string()),
                "reply",
            )
            .map_err(|e| BotError::Db(e))?;

        let tagger_name = &msg.author.name;
        let tagger_mention = msg.author.id.mention().to_string();
        let tagger_content = utils::strip_mentions(&msg.content);
        let target_name = &replied_msg.author.name;
        let target_mention = replied_msg.author.id.mention().to_string();
        let target_content = &replied_msg.content;

        let response = crate::agents::roast_reply(
            &ctx.llm_service,
            ctx.serenity_ctx.clone().into(),
            msg.channel_id,
            tagger_name,
            &tagger_mention,
            &tagger_content,
            target_name,
            &target_mention,
            target_content,
            &channel_ctx,
        )
        .await?;

        Ok(Some(response))
    }
}
