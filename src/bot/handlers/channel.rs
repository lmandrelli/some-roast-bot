use poise::serenity_prelude::{self as serenity};
use std::sync::Arc;

use crate::bot::Error;
use crate::bot::context;

pub fn extract_mentioned_user(content: &str) -> Option<String> {
    let re = regex::Regex::new(r"<@!?(\d+)>").ok()?;
    let caps = re.captures(content)?;
    caps.get(1).map(|m| m.as_str().to_string())
}

pub async fn handle_channel(
    ctx: &serenity::Context,
    msg: &serenity::Message,
) -> Result<String, Error> {
    tracing::info!("Priority 3: Channel roast triggered by {}", msg.author.name);

    let triggerer_id = msg.author.id.to_string();
    let channel_ctx = context::fetch_channel_context(ctx, msg.channel_id, msg.id, 20, true).await?;

    let mut response =
        crate::agents::roast_channel(Arc::new(ctx.clone()), msg.channel_id, &channel_ctx).await?;

    for _ in 0..3 {
        if let Some(target_id) = extract_mentioned_user(&response) {
            if target_id != triggerer_id {
                crate::memory::record_roast(&triggerer_id, Some(&target_id), "channel");
                return Ok(response);
            }
        }

        let retry_context = format!(
            "{}\n\nTu as mentionné la mauvaise personne. Choisis quelqu'un d'autre que <@{triggerer_id}> cette fois.",
            channel_ctx.to_string()
        );
        response = crate::agents::roast_channel_with_context(
            Arc::new(ctx.clone()),
            msg.channel_id,
            &retry_context,
        )
        .await?;
    }

    if let Some(target_id) = extract_mentioned_user(&response) {
        crate::memory::record_roast(&triggerer_id, Some(&target_id), "channel");
    }

    Ok(response)
}
