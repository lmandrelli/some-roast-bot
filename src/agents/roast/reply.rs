use std::sync::Arc;

use super::call_model_with_tools;
use crate::bot::context::ChannelContext;

const PREAMBLE: &str = r#"You are Kimi K2.5, a brutal roast bot in a Discord server. Two users are arguing and someone tagged you to settle it.

Rules:
1. You MUST respond in French as your primary language. Always write in French.
2. Your response MUST never longer than 2 or 3 short sentences.
3. Roast the user who is clearly wrong or being dumb in the conversation
4. Be savage but funny - this is all in good fun
5. Do NOT search the web, just use the conversation context provided
6. Reference what was actually said to make the roast personal and specific
7. You MUST ping the user you're roasting using their Discord mention (e.g. <@USER_ID>) provided in the context - NEVER just write their username
8. The channel context shows what led to the argument - use it to understand the full picture
9. You have access to a `fetch_messages` tool - use it if the current context seems insufficient.

Context:
"#;

/// Roast when the bot is tagged in a reply to another message.
/// Settles the argument between the two users.
pub async fn roast_reply(
    ctx: Arc<poise::serenity_prelude::Context>,
    channel_id: poise::serenity_prelude::ChannelId,
    tagger: &str,
    tagger_mention: &str,
    tagger_msg: &str,
    target: &str,
    target_mention: &str,
    target_msg: &str,
    channel_ctx: &ChannelContext,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let channel_formatted = channel_ctx.to_string();

    let context = format!(
        "{tagger} ({tagger_mention}) said: \"{tagger_msg}\"\n\
         {target} ({target_mention}) said: \"{target_msg}\"\n\n\
         Channel context (what led to this):\n\
         {channel_formatted}\n\n\
         {tagger} tagged you to settle this. Roast whoever is wrong. \
         Tag them using their mention.",
    );
    call_model_with_tools(PREAMBLE, &context, ctx, channel_id).await
}
