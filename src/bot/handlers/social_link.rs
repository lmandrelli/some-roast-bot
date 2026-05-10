use crate::bot::Error;
use crate::fixers;
use poise::serenity_prelude as serenity;

/// Detect social-media links in a message, suppress the original embeds,
/// and reply with fixed (embed-friendly) URLs.
///
/// Returns `true` when at least one link was handled.
pub async fn handle_social_links(
    ctx: &serenity::Context,
    msg: &serenity::Message,
) -> Result<bool, Error> {
    let fixed = fixers::fix_links(&msg.content).await;
    if fixed.is_empty() {
        return Ok(false);
    }

    // Suppress the ugly original embeds on the user's message.
    // This requires the *Manage Messages* permission.
    let suppress_edit = serenity::EditMessage::new().suppress_embeds(true);
    if let Err(e) = msg
        .channel_id
        .edit_message(&ctx.http, msg.id, suppress_edit)
        .await
    {
        tracing::warn!("Failed to suppress embeds for msg {}: {}", msg.id, e);
    }

    let urls = fixed
        .iter()
        .map(|l| l.fixed_url.clone())
        .collect::<Vec<_>>()
        .join(" ");

    let reply_text = format!("{} posted: {}", msg.author.name, urls);

    let bot_reply = msg.reply(&ctx.http, reply_text).await?;

    // Persist the mapping so we can update this reply on message edits.
    crate::memory::store_link_fix_reply(
        &msg.id.to_string(),
        &bot_reply.id.to_string(),
        &msg.channel_id.to_string(),
    );

    // Stats
    crate::memory::record_roast(&msg.author.id.to_string(), None, "social_link");

    Ok(true)
}

/// React to a message edit: update or delete our previous fix reply.
pub async fn handle_message_update(
    ctx: &serenity::Context,
    event: &serenity::MessageUpdateEvent,
) -> Result<(), Error> {
    let new_content = match &event.content {
        Some(c) => c,
        None => return Ok(()),
    };

    let original_msg_id = event.id.to_string();

    let bot_reply_id = match crate::memory::get_link_fix_reply(&original_msg_id) {
        Some(id) => id,
        None => return Ok(()),
    };

    let fixed = fixers::fix_links(new_content).await;

    let channel_id = event.channel_id;

    if fixed.is_empty() {
        // User removed all links — delete our reply.
        if let Err(e) = channel_id.delete_message(&ctx.http, bot_reply_id).await {
            tracing::warn!("Failed to delete link-fix reply {}: {}", bot_reply_id, e);
        }
        crate::memory::remove_link_fix_reply(&original_msg_id);
        return Ok(());
    }

    let urls = fixed
        .iter()
        .map(|l| l.fixed_url.clone())
        .collect::<Vec<_>>()
        .join(" ");

    let author_name = event
        .author
        .as_ref()
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "Someone".to_string());

    let now = chrono::Utc::now().format("%H:%M:%S");
    let new_text = format!("{} posted: {} (modified at: {})", author_name, urls, now);

    let edit = serenity::EditMessage::new().content(new_text);
    if let Err(e) = channel_id.edit_message(&ctx.http, bot_reply_id, edit).await {
        tracing::warn!("Failed to edit link-fix reply {}: {}", bot_reply_id, e);
    }

    Ok(())
}
