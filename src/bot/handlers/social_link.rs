use async_trait::async_trait;

use crate::bot::handler::{HandlerContext, MessageHandler};
use crate::error::BotError;
use crate::fixers;

/// Handler for social-media link fixes.
pub struct SocialLinkHandler;

#[async_trait]
impl MessageHandler for SocialLinkHandler {
    fn name(&self) -> &'static str {
        "social_link"
    }

    fn priority(&self) -> u8 {
        0
    }

    async fn can_handle(&self, ctx: &HandlerContext<'_>) -> bool {
        // Cheap check: are there any fixable links in the message?
        !fixers::fix_links(&ctx.message.content).await.is_empty()
    }

    async fn handle(&self, ctx: &HandlerContext<'_>) -> Result<Option<String>, BotError> {
        let msg = ctx.message;
        let fixed = fixers::fix_links(&msg.content).await;
        if fixed.is_empty() {
            return Ok(None);
        }

        // Delete the user's original message
        if let Err(e) = msg.delete(&ctx.serenity_ctx.http).await {
            tracing::warn!("Failed to delete original msg {}: {}", msg.id, e);
        }

        let urls = fixed
            .iter()
            .map(|l| l.fixed_url.clone())
            .collect::<Vec<_>>()
            .join(" ");

        let header_text = format!("{} posted :", msg.author.name);

        msg.channel_id
            .say(&ctx.serenity_ctx.http, header_text)
            .await?;
        msg.channel_id.say(&ctx.serenity_ctx.http, urls).await?;

        ctx.memory
            .record_roast(&msg.author.id.to_string(), None, "social_link")
            .map_err(|e| BotError::Db(e))?;

        Ok(Some(String::new())) // empty = already sent our own messages
    }
}
