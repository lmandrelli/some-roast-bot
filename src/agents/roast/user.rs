use std::sync::Arc;

use super::call_model_with_tools;
use crate::bot::context::{ChannelContext, FormattedMessage};

const PREAMBLE: &str = r#"You are Kimi K2.5, a brutal roast bot in a Discord server. Someone tagged you and pointed at another user to roast.

Rules:
1. You MUST respond in French as your primary language. Always write in French.
2. Your response MUST never longer than 2 or 3 short sentences.
3. Analyze the tagged user's recent messages and roast them based on what they said
4. Be savage but funny - this is all in good fun
5. Do NOT search the web, just use the conversation context provided
6. Reference what they actually said to make the roast specific
7. You MUST start your message by pinging the target user using their Discord mention (e.g. <@USER_ID>) provided in the context - NEVER just write their username
8. The channel context shows what others were saying around the target user's messages for additional context
9. You have access to a `fetch_messages` tool - use it if the current context seems insufficient.

Context:
"#;

/// Roast when the bot is tagged alongside another user.
/// Analyzes the target user's recent messages and roasts them.
pub async fn roast_user(
    ctx: Arc<poise::serenity_prelude::Context>,
    channel_id: poise::serenity_prelude::ChannelId,
    tagger: &str,
    target: &str,
    target_mention: &str,
    target_messages: &[FormattedMessage],
    channel_ctx: &ChannelContext,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let target_formatted = crate::bot::context::format_user_messages(target_messages);
    let channel_formatted = channel_ctx.to_string();

    let context = format!(
        "{tagger} wants you to roast {target} ({target_mention}).\n\n\
         {target}'s recent messages:\n\
         {target_formatted}\n\n\
         Channel context:\n\
         {channel_formatted}\n\n\
         Tag them using their mention: {target_mention}",
    );
    call_model_with_tools(PREAMBLE, &context, ctx, channel_id).await
}
