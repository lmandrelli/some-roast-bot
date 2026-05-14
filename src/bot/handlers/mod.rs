use std::sync::Arc;

use poise::serenity_prelude::{self as serenity, FullEvent};

use crate::bot::Data;
use crate::bot::handler::HandlerContext;
use crate::bot::pipeline::HandlerPipeline;
use crate::bot::utils;
use crate::error::BotError;

mod channel;
mod microsoft;
mod quoi;
mod reply;
mod social_link;
mod truth;
mod user;

use channel::ChannelHandler;
use microsoft::MicrosoftHandler;
use quoi::QuoiHandler;
use reply::ReplyHandler;
use social_link::SocialLinkHandler;
use truth::TruthHandler;
use user::UserHandler;

/// Build the default handler pipeline.
pub fn build_pipeline() -> HandlerPipeline {
    let mut pipeline = HandlerPipeline::new();
    pipeline.register(Box::new(SocialLinkHandler));
    pipeline.register(Box::new(QuoiHandler));
    pipeline.register(Box::new(MicrosoftHandler));
    pipeline.register(Box::new(TruthHandler));
    pipeline.register(Box::new(ReplyHandler));
    pipeline.register(Box::new(UserHandler));
    pipeline.register(Box::new(ChannelHandler));
    pipeline
}

/// Poise event handler that listens for messages and dispatches through the handler pipeline.
pub async fn event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, BotError>,
    data: &Data,
) -> Result<(), BotError> {
    if let FullEvent::Message { new_message } = event {
        // Ignore messages from bots to avoid loops
        if new_message.author.bot {
            return Ok(());
        }

        let mentions_me = new_message.mentions_me(&ctx.http).await.unwrap_or(false);
        let bot_id = ctx.http.get_current_user().await?.id;

        let handler_ctx = HandlerContext {
            serenity_ctx: ctx,
            message: new_message,
            mentions_me,
            bot_id,
            memory: Arc::clone(&data.memory),
            llm_service: Arc::clone(&data.llm_service),
        };

        // Show typing indicator while generating response
        let typing = new_message.channel_id.start_typing(&ctx.http);

        let result = data.pipeline.run(&handler_ctx).await;

        // Stop typing
        drop(typing);

        match result {
            Ok(Some(response)) => {
                if !response.is_empty() {
                    let response = utils::strip_self_mentions(ctx, &response).await;
                    utils::send_roast(ctx, new_message.channel_id, &response).await?;
                }
            }
            Ok(None) => {
                // No handler matched — nothing to do
            }
            Err(e) => {
                tracing::error!("Roast failed: {:?}", e);
                let error_response = crate::error::discord_error_response(&e);
                new_message.reply(&ctx.http, &error_response).await?;
            }
        }
    }

    Ok(())
}
