mod channel;
mod microsoft;
mod reply;
mod truth;
mod user;

use crate::bot::{Data, Error};
use futures_util::future::BoxFuture;
use poise::serenity_prelude::{self as serenity, FullEvent};

pub use channel::handle_channel;
pub use microsoft::handle_microsoft;
pub use reply::handle_reply;
pub use truth::handle_truth;
pub use user::handle_user;

/// Poise event handler that listens for messages where the bot is tagged.
///
/// Priority logic:
/// 0a. Message contains Microsoft/Windows keywords -> roast with Microslop/Windaube
/// 1. Tagged + replying to another message -> roast whoever is wrong between the two users
/// 2. Tagged + another user also tagged -> fetch last 5 messages from that user, roast them
/// 3a. Message contains "is this true?" -> judge the claim using channel context
/// 3b. Tagged alone -> fetch last 10 channel messages, pick and roast someone
pub fn event_handler<'a>(
    ctx: &'a serenity::Context,
    event: &'a FullEvent,
    _framework: poise::FrameworkContext<'a, Data, Error>,
    _data: &'a Data,
) -> BoxFuture<'a, Result<(), Error>> {
    Box::pin(async move {
        if let FullEvent::Message { new_message } = event {
            // Ignore messages from bots to avoid loops
            if new_message.author.bot {
                return Ok(());
            }

            // Check if the bot was mentioned
            let mentions_me: bool = new_message.mentions_me(&ctx.http).await.unwrap_or(false);
            if !mentions_me {
                return Ok(());
            }

            tracing::info!(
                "Bot was mentioned by {} in channel {}",
                new_message.author.name,
                new_message.channel_id
            );

            // Show typing indicator while we generate the response
            let typing = new_message.channel_id.start_typing(&ctx.http);

            let result = handle_mention(ctx, new_message).await;

            // Stop typing
            drop(typing);

            match result {
                Ok(response) => {
                    new_message.reply(&ctx.http, &response).await?;
                }
                Err(e) => {
                    tracing::error!("Roast failed: {:?}", e);
                    new_message
                        .reply(&ctx.http, "Even I can't roast this situation... something broke.")
                        .await?;
                }
            }
        }

        Ok(())
    })
}

async fn handle_mention(
    ctx: &serenity::Context,
    msg: &serenity::Message,
) -> Result<String, Error> {
    let bot_id = ctx.http.get_current_user().await?.id;

    // Priority 0: Message contains Microsoft/Windows keywords
    if microsoft::contains_microsoft_keywords(&msg.content) {
        return handle_microsoft(msg).await;
    }

    // Priority 1: Bot is tagged in a reply to another message
    if let Some(ref replied_msg) = msg.referenced_message {
        return handle_reply(msg, replied_msg).await;
    }

    // Priority 2: Bot is tagged alongside another user
    let other_mentions: Vec<_> = msg
        .mentions
        .iter()
        .filter(|u| u.id != bot_id && !u.bot)
        .collect();

    if let Some(target_user) = other_mentions.first() {
        return handle_user(ctx, msg, target_user).await;
    }
    
    // Priority 3a: Message contains "is this true?"
    if truth::contains_truth_question(&msg.content) {
        return handle_truth(ctx, msg).await;
    }

    // Priority 3b: Bot tagged alone - roast based on recent channel context
    handle_channel(ctx, msg).await
}

/// Remove bot mentions from message content so the prompt is cleaner.
fn strip_mentions(content: &str) -> String {
    // Discord mentions look like <@123456789> or <@!123456789>
    let re = regex::Regex::new(r"<@!?\d+>").unwrap();
    re.replace_all(content, "").trim().to_string()
}
