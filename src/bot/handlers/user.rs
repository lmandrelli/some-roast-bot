use poise::serenity_prelude::{self as serenity, Mentionable};

use crate::bot::Error;

/// Priority 2: Bot is tagged alongside another user.
/// Fetches their recent messages and roasts them.
pub async fn handle_user(
    ctx: &serenity::Context,
    msg: &serenity::Message,
    target_user: &serenity::User,
) -> Result<String, Error> {
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
    let target_mention = target_user.id.mention().to_string();

    crate::agents::roast_user(tagger_name, target_name, &target_mention, &target_messages).await
}
