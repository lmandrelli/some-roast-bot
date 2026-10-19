use async_trait::async_trait;
use poise::serenity_prelude::{CreateMessage, MessageFlags};

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
        // Cheap check (regex only): are there any fixable links in the message?
        // The network preflight runs once, in `handle`.
        fixers::has_fixable_link(&ctx.message.content)
    }

    async fn handle(&self, ctx: &HandlerContext<'_>) -> Result<Option<String>, BotError> {
        let msg = ctx.message;
        let outcome = fixers::fix_links(&msg.content).await;

        for link in &outcome.fixed {
            tracing::info!(
                "Fixed {} link {} -> {}",
                link.platform,
                link.original_url,
                link.fixed_url
            );
        }
        for link in &outcome.failed {
            tracing::warn!(
                "Could not fix {} link {}: {}",
                link.platform,
                link.original_url,
                link.reason
            );
        }

        let failure = (!outcome.failed.is_empty()).then(|| {
            let summary = outcome
                .failed
                .iter()
                .map(|l| format!("{} {}: {}", l.platform, l.original_url, l.reason))
                .collect::<Vec<_>>()
                .join(" | ");
            BotError::LinkFix(summary)
        });

        let fixed = outcome.fixed;
        if fixed.is_empty() {
            // Nothing to repost: keep the original message and report the failure.
            return match failure {
                Some(e) => Err(e),
                None => Ok(None),
            };
        }

        // Delete the user's original message, unless some links could not be
        // fixed: the original is then the only place those links remain.
        if failure.is_none()
            && let Err(e) = msg.delete(&ctx.serenity_ctx.http).await
        {
            tracing::warn!("Failed to delete original msg {}: {}", msg.id, e);
        }

        let urls = fixed
            .iter()
            .map(|l| l.fixed_url.clone())
            .collect::<Vec<_>>()
            .join(" ");

        let header_text = format!("<@{}> posted :", msg.author.id);

        msg.channel_id
            .send_message(
                &ctx.serenity_ctx.http,
                CreateMessage::new()
                    .content(header_text)
                    .flags(MessageFlags::SUPPRESS_NOTIFICATIONS),
            )
            .await?;
        msg.channel_id.say(&ctx.serenity_ctx.http, urls).await?;

        if fixed.iter().any(|l| l.fallback) {
            msg.channel_id
                .say(
                    &ctx.serenity_ctx.http,
                    crate::error::link_fallback_response(),
                )
                .await?;
        }

        ctx.memory
            .record_roast(&msg.author.id.to_string(), None, "social_link")
            .map_err(|e| BotError::Db(e))?;

        if let Some(e) = failure {
            return Err(e);
        }

        Ok(Some(String::new())) // empty = already sent our own messages
    }
}
