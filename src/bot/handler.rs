use crate::bot::{Data, Error};
use futures_util::future::BoxFuture;
use poise::serenity_prelude::{self as serenity, FullEvent, Mentionable};

/// Poise event handler that listens for messages where the bot is tagged.
///
/// Priority logic:
/// 1. Tagged + replying to another message -> roast whoever is wrong between the two users
/// 2. Tagged + another user also tagged -> fetch last 5 messages from that user, roast them
/// 3. Tagged alone -> fetch last 10 channel messages, pick and roast someone
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

    // Priority 1: Bot is tagged in a reply to another message
    if let Some(ref replied_msg) = msg.referenced_message {
        tracing::info!("Priority 1: Reply roast between {} and {}", msg.author.name, replied_msg.author.name);

        let tagger_name = &msg.author.name;
        let tagger_content = strip_mentions(&msg.content);
        let target_name = &replied_msg.author.name;
        let target_content = &replied_msg.content;

        return crate::agents::roast_reply(
            tagger_name,
            &tagger_content,
            target_name,
            target_content,
        )
        .await;
    }

    // Priority 2: Bot is tagged alongside another user
    let other_mentions: Vec<_> = msg
        .mentions
        .iter()
        .filter(|u| u.id != bot_id && !u.bot)
        .collect();

    if let Some(target_user) = other_mentions.first() {
        tracing::info!(
            "Priority 2: User roast - {} wants to roast {}",
            msg.author.name,
            target_user.name
        );

        // Fetch last 25 messages from the channel and filter by the target user (up to 5)
        let builder = serenity::builder::GetMessages::new()
            .before(msg.id)
            .limit(25);
        let messages = msg.channel_id.messages(&ctx.http, builder).await?;

        let target_messages: Vec<(String, String)> = messages
            .iter()
            .filter(|m| m.author.id == target_user.id)
            .take(5)
            .map(|m| (m.author.name.clone(), m.content.clone()))
            .collect();

        let tagger_name = &msg.author.name;
        let target_name = &target_user.name;

        return crate::agents::roast_user(tagger_name, target_name, &target_messages).await;
    }

    // Priority 3: Bot tagged alone - roast based on recent channel context
    tracing::info!(
        "Priority 3: Channel roast triggered by {}",
        msg.author.name
    );

    let builder = serenity::builder::GetMessages::new()
        .before(msg.id)
        .limit(10);
    let messages = msg.channel_id.messages(&ctx.http, builder).await?;

    let context_messages: Vec<(String, String, String)> = messages
        .iter()
        .filter(|m| !m.author.bot)
        .map(|m| {
            (
                m.author.name.clone(),
                m.author.id.mention().to_string(),
                m.content.clone(),
            )
        })
        .collect();

    crate::agents::roast_channel(&context_messages).await
}

/// Remove bot mentions from message content so the prompt is cleaner.
fn strip_mentions(content: &str) -> String {
    // Discord mentions look like <@123456789> or <@!123456789>
    let re = regex::Regex::new(r"<@!?\d+>").unwrap();
    re.replace_all(content, "").trim().to_string()
}
