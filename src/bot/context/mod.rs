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
    #[allow(dead_code)]
    pub total_messages: usize,
}

impl ChannelContext {
    pub fn to_string(&self) -> String {
        formatter::format_channel_context(self)
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
        content: trigger_msg.content.clone(),
    };

    let mut formatted: Vec<FormattedMessage> = messages
        .iter()
        .filter(|m| !filter_bot || !m.author.bot)
        .map(|m| FormattedMessage {
            timestamp: m.timestamp,
            author_name: m.author.name.clone(),
            author_mention_id: m.author.id.to_string(),
            content: m.content.clone(),
        })
        .collect();

    formatted.reverse();
    let total = formatted.len();

    Ok(ChannelContext {
        trigger: Some(trigger),
        messages: formatted,
        total_messages: total,
    })
}

pub async fn fetch_user_context(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    trigger_msg: &serenity::Message,
    target_user_id: serenity::UserId,
) -> Result<(Vec<FormattedMessage>, ChannelContext), BotError> {
    let builder = serenity::builder::GetMessages::new()
        .before(trigger_msg.id)
        .limit(CONTEXT_FETCH_LIMIT as u8);
    let messages = channel_id.messages(&ctx.http, builder).await?;

    let trigger = FormattedMessage {
        timestamp: trigger_msg.timestamp,
        author_name: trigger_msg.author.name.clone(),
        author_mention_id: trigger_msg.author.id.to_string(),
        content: trigger_msg.content.clone(),
    };

    let target_messages: Vec<FormattedMessage> = messages
        .iter()
        .filter(|m| m.id != trigger_msg.id)
        .filter(|m| m.author.id == target_user_id)
        .take(5)
        .map(|m| FormattedMessage {
            timestamp: m.timestamp,
            author_name: m.author.name.clone(),
            author_mention_id: m.author.id.to_string(),
            content: m.content.clone(),
        })
        .collect();

    let all_formatted: Vec<FormattedMessage> = messages
        .iter()
        .filter(|m| m.id != trigger_msg.id)
        .filter(|m| !m.author.bot)
        .map(|m| FormattedMessage {
            timestamp: m.timestamp,
            author_name: m.author.name.clone(),
            author_mention_id: m.author.id.to_string(),
            content: m.content.clone(),
        })
        .collect();

    let mut history = all_formatted;
    history.reverse();
    let total = history.len();

    Ok((
        target_messages,
        ChannelContext {
            trigger: Some(trigger),
            messages: history,
            total_messages: total,
        },
    ))
}
