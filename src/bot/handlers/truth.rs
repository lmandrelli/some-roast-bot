use async_trait::async_trait;

use crate::bot::context;
use crate::bot::handler::{HandlerContext, MessageHandler};
use crate::bot::utils;
use crate::error::BotError;

/// Handler for "is this true?" / "is that true?" truth checks.
pub struct TruthHandler;

fn contains_truth_question(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("is this true?")
        || lower.contains("is this true ?")
        || lower.contains("is that true?")
        || lower.contains("is that true ?")
}

#[async_trait]
impl MessageHandler for TruthHandler {
    fn name(&self) -> &'static str {
        "truth"
    }

    fn priority(&self) -> u8 {
        3
    }

    async fn can_handle(&self, ctx: &HandlerContext<'_>) -> bool {
        contains_truth_question(&ctx.message.content)
    }

    async fn handle(&self, ctx: &HandlerContext<'_>) -> Result<Option<String>, BotError> {
        let msg = ctx.message;
        tracing::info!(
            "Truth check triggered by {} in channel {}",
            msg.author.name,
            msg.channel_id
        );

        let channel_ctx =
            context::fetch_channel_context(ctx.serenity_ctx, msg.channel_id, msg.id, 20, true)
                .await?;

        let response = crate::agents::roast_truth(
            &ctx.llm_service,
            ctx.serenity_ctx.clone().into(),
            msg.channel_id,
            &channel_ctx,
        )
        .await?;

        if let Some(target_id) = utils::extract_mentioned_user(&response) {
            ctx.memory
                .record_roast(&msg.author.id.to_string(), Some(&target_id), "truth")
                .map_err(|e| BotError::Db(e))?;
        } else {
            ctx.memory
                .record_roast(&msg.author.id.to_string(), None, "truth")
                .map_err(|e| BotError::Db(e))?;
        }

        Ok(Some(response))
    }
}
