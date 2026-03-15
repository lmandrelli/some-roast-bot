use poise::serenity_prelude::{self as serenity, Mentionable};

use crate::bot::Error;

use super::strip_mentions;

/// Priority 1: Bot is tagged in a reply to another message.
/// Settles the argument between the two users.
pub async fn handle_reply(
    msg: &serenity::Message,
    replied_msg: &serenity::Message,
) -> Result<String, Error> {
    tracing::info!(
        "Priority 1: Reply roast between {} and {}",
        msg.author.name,
        replied_msg.author.name
    );

    let tagger_name = &msg.author.name;
    let tagger_mention = msg.author.id.mention().to_string();
    let tagger_content = strip_mentions(&msg.content);
    let target_name = &replied_msg.author.name;
    let target_mention = replied_msg.author.id.mention().to_string();
    let target_content = &replied_msg.content;

    crate::agents::roast_reply(
        tagger_name,
        &tagger_mention,
        &tagger_content,
        target_name,
        &target_mention,
        target_content,
    )
    .await
}
