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

/// Poise event handler that listens for messages.
///
/// Priority logic:
/// 0a. Message contains Microsoft/Windows keywords (no mention required) -> roast with Microslop/Windaube
/// 3a. Message contains "is this true?" / "is that true?" (no mention required) -> judge the claim
/// --- below requires bot mention ---
/// 1. Tagged + replying to another message -> roast whoever is wrong between the two users
/// 2. Tagged + another user also tagged -> fetch last 5 messages from that user, roast them
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

            // Priority 0a: Microsoft/Windows keywords (no mention required)
            let has_microsoft = microsoft::contains_microsoft_keywords(&new_message.content);

            // Priority 3a: "is this true?" / "is that true?" (no mention required)
            let has_truth = truth::contains_truth_question(&new_message.content);

            // Check if the bot was mentioned
            let mentions_me: bool = new_message.mentions_me(&ctx.http).await.unwrap_or(false);

            // If no passive trigger and bot is not mentioned, ignore the message
            if !has_microsoft && !has_truth && !mentions_me {
                return Ok(());
            }

            tracing::info!(
                "Handling message from {} in channel {} (mentioned={}, microsoft={}, truth={})",
                new_message.author.name,
                new_message.channel_id,
                mentions_me,
                has_microsoft,
                has_truth,
            );

            // Show typing indicator while we generate the response
            let typing = new_message.channel_id.start_typing(&ctx.http);

            let result = handle_message(ctx, new_message, mentions_me, has_microsoft, has_truth).await;

            // Stop typing
            drop(typing);

            match result {
                Ok(response) => {
                    // Guardrail: strip any self-mentions so the bot never pings itself
                    let response = strip_self_mentions(ctx, &response).await;
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

async fn handle_message(
    ctx: &serenity::Context,
    msg: &serenity::Message,
    mentions_me: bool,
    has_microsoft: bool,
    has_truth: bool,
) -> Result<String, Error> {
    // Priority 0a: Message contains Microsoft/Windows keywords (no mention required)
    if has_microsoft {
        return handle_microsoft(msg).await;
    }

    // Priority 3a: Message contains "is this true?" / "is that true?" (no mention required)
    if has_truth {
        return handle_truth(ctx, msg).await;
    }

    // Below this point, the bot must be mentioned
    if !mentions_me {
        return Err("No trigger matched".into());
    }

    let bot_id = ctx.http.get_current_user().await?.id;

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

    // Priority 3b: Bot tagged alone - roast based on recent channel context
    handle_channel(ctx, msg).await
}

/// Remove bot mentions from message content so the prompt is cleaner.
fn strip_mentions(content: &str) -> String {
    // Discord mentions look like <@123456789> or <@!123456789>
    let re = regex::Regex::new(r"<@!?\d+>").unwrap();
    re.replace_all(content, "").trim().to_string()
}

/// Guardrail: replace the bot's own mention with `<filtered>` so it
/// never pings itself.
async fn strip_self_mentions(ctx: &serenity::Context, content: &str) -> String {
    let bot_id = match ctx.http.get_current_user().await {
        Ok(user) => user.id.to_string(),
        Err(_) => return content.to_string(),
    };
    // Matches <@BOT_ID> and <@!BOT_ID>
    let pattern = format!(r"<@!?{bot_id}>");
    let re = regex::Regex::new(&pattern).unwrap();
    re.replace_all(content, "<filtered>").to_string()
}
