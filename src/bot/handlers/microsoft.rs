use async_trait::async_trait;
use poise::serenity_prelude::Mentionable;

use crate::bot::context;
use crate::bot::handler::{HandlerContext, MessageHandler};
use crate::bot::utils;
use crate::error::BotError;

/// Handler for Microsoft/Windows keyword roasts.
pub struct MicrosoftHandler;

fn contains_microsoft_keywords(content: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("microsoft") || lower.contains("windows")
}

#[async_trait]
impl MessageHandler for MicrosoftHandler {
    fn name(&self) -> &'static str {
        "microsoft"
    }

    fn priority(&self) -> u8 {
        2
    }

    async fn can_handle(&self, ctx: &HandlerContext<'_>) -> bool {
        contains_microsoft_keywords(&ctx.message.content)
    }

    async fn handle(&self, ctx: &HandlerContext<'_>) -> Result<Option<String>, BotError> {
        let msg = ctx.message;
        tracing::info!(
            "Microsoft/Windows detected in message from {}",
            msg.author.name,
        );

        let channel_ctx =
            context::fetch_channel_context(ctx.serenity_ctx, msg.channel_id, msg.id, 5, true)
                .await?;

        let clean_content = utils::strip_mentions(&msg.content);

        ctx.memory
            .record_roast(
                &msg.author.id.to_string(),
                Some(&msg.author.id.to_string()),
                "microsoft",
            )
            .map_err(|e| BotError::Db(e))?;

        ctx.memory
            .increment_microsoft_count()
            .map_err(|e| BotError::Db(e))?;

        let response = crate::agents::roast_microsoft(
            &ctx.llm_service,
            ctx.memory.as_ref(),
            &msg.author.name,
            &msg.author.id.mention().to_string(),
            &clean_content,
            &channel_ctx,
        )
        .await?;

        Ok(Some(response))
    }
}
