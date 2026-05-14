use poise::serenity_prelude as serenity;

use crate::error::BotError;

pub mod formatter;

/// Total messages in context including trigger.
const CONTEXT_TOTAL_LIMIT: usize = 15;
/// Messages to fetch before the trigger message.
const CONTEXT_FETCH_LIMIT: usize = CONTEXT_TOTAL_LIMIT - 1;

#[derive(Debug, Clone)]
pub struct FormattedMessage {
    pub timestamp: serenity::Timestamp,
    pub author_name: String,
    pub author_mention_id: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ChannelContext {
    pub trigger: Option<FormattedMessage>,
    pub messages: Vec<FormattedMessage>,
}

impl std::fmt::Display for ChannelContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&formatter::format_channel_context(self))
    }
}

pub async fn fetch_channel_context(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    trigger_msg: &serenity::Message,
    filter_bot: bool,
) -> Result<ChannelContext, BotError> {
    let builder = serenity::builder::GetMessages::new()
        .before(trigger_msg.id)
        .limit(CONTEXT_FETCH_LIMIT as u8);
    let messages = channel_id.messages(&ctx.http, builder).await?;

    let trigger = FormattedMessage {
        timestamp: trigger_msg.timestamp,
        author_name: trigger_msg.author.name.clone(),
        author_mention_id: trigger_msg.author.id.to_string(),
        content: crate::bot::utils::strip_mentions(&trigger_msg.content),
    };

    let mut formatted: Vec<FormattedMessage> = messages
        .iter()
        .filter(|m| m.id != trigger_msg.id)
        .filter(|m| !filter_bot || !m.author.bot)
        .map(|m| FormattedMessage {
            timestamp: m.timestamp,
            author_name: m.author.name.clone(),
            author_mention_id: m.author.id.to_string(),
            content: crate::bot::utils::strip_mentions(&m.content),
        })
        .collect();

    formatted.reverse();

    Ok(ChannelContext {
        trigger: Some(trigger),
        messages: formatted,
    })
}
