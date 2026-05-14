use poise::serenity_prelude as serenity;
use std::time::Duration;

use crate::error::BotError;

pub mod formatter;

const CONVERSATION_GAP_MINUTES: u64 = 5;
const INITIAL_FETCH_LIMIT: usize = 20;
const MAX_FETCH_LIMIT: usize = 50;

#[derive(Debug, Clone)]
pub struct FormattedMessage {
    pub timestamp: serenity::Timestamp,
    pub author_name: String,
    pub content: String,
    pub reply_to: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConversationThread {
    pub messages: Vec<FormattedMessage>,
    pub gap_before: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct ChannelContext {
    pub threads: Vec<ConversationThread>,
    pub total_messages: usize,
    pub can_fetch_more: bool,
}

impl ChannelContext {
    pub fn to_string(&self) -> String {
        formatter::format_channel_context(self)
    }
}

pub async fn fetch_channel_context(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    before_msg_id: serenity::MessageId,
    limit: usize,
    filter_bot: bool,
) -> Result<ChannelContext, BotError> {
    let clamped_limit = limit.min(INITIAL_FETCH_LIMIT);

    let builder = serenity::builder::GetMessages::new()
        .before(before_msg_id)
        .limit(clamped_limit as u8);
    let messages = channel_id.messages(&ctx.http, builder).await?;

    let mut formatted: Vec<FormattedMessage> = messages
        .iter()
        .filter(|m| !filter_bot || !m.author.bot)
        .map(|m| {
            let reply_to = m
                .referenced_message
                .as_ref()
                .map(|ref_msg| ref_msg.author.name.clone());

            FormattedMessage {
                timestamp: m.timestamp,
                author_name: m.author.name.clone(),
                content: m.content.clone(),
                reply_to,
            }
        })
        .collect();

    formatted.reverse();

    let threads = split_into_threads(&formatted);
    let can_fetch_more = formatted.len() >= clamped_limit && clamped_limit < MAX_FETCH_LIMIT;

    Ok(ChannelContext {
        threads,
        total_messages: formatted.len(),
        can_fetch_more,
    })
}

pub async fn fetch_user_context(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    before_msg_id: serenity::MessageId,
    target_user_id: serenity::UserId,
    fetch_limit: usize,
    max_user_messages: usize,
) -> Result<(Vec<FormattedMessage>, ChannelContext), BotError> {
    let builder = serenity::builder::GetMessages::new()
        .before(before_msg_id)
        .limit(fetch_limit as u8);
    let messages = channel_id.messages(&ctx.http, builder).await?;

    let target_messages: Vec<FormattedMessage> = messages
        .iter()
        .filter(|m| m.author.id == target_user_id)
        .take(max_user_messages)
        .map(|m| FormattedMessage {
            timestamp: m.timestamp,
            author_name: m.author.name.clone(),
            content: m.content.clone(),
            reply_to: m
                .referenced_message
                .as_ref()
                .map(|ref_msg| ref_msg.author.name.clone()),
        })
        .collect();

    let all_formatted: Vec<FormattedMessage> = messages
        .iter()
        .filter(|m| !m.author.bot)
        .map(|m| FormattedMessage {
            timestamp: m.timestamp,
            author_name: m.author.name.clone(),
            content: m.content.clone(),
            reply_to: m
                .referenced_message
                .as_ref()
                .map(|ref_msg| ref_msg.author.name.clone()),
        })
        .collect();

    let threads = split_into_threads(&all_formatted);

    Ok((
        target_messages,
        ChannelContext {
            threads,
            total_messages: all_formatted.len(),
            can_fetch_more: all_formatted.len() >= fetch_limit && fetch_limit < MAX_FETCH_LIMIT,
        },
    ))
}

fn split_into_threads(messages: &[FormattedMessage]) -> Vec<ConversationThread> {
    if messages.is_empty() {
        return vec![];
    }

    let mut threads: Vec<ConversationThread> = vec![];
    let mut current_thread: Vec<FormattedMessage> = vec![messages[0].clone()];

    for window in messages.windows(2) {
        let prev = &window[0];
        let next = &window[1];

        let gap = match next
            .timestamp
            .signed_duration_since(*prev.timestamp)
            .num_seconds()
        {
            secs if secs > 0 => Duration::from_secs(secs as u64),
            _ => Duration::ZERO,
        };

        if gap.as_secs() >= CONVERSATION_GAP_MINUTES * 60 {
            threads.push(ConversationThread {
                messages: std::mem::take(&mut current_thread),
                gap_before: None,
            });
            current_thread.push(next.clone());
            if let Some(last) = threads.last_mut() {
                last.gap_before = Some(gap);
            }
        } else {
            current_thread.push(next.clone());
        }
    }

    if !current_thread.is_empty() {
        threads.push(ConversationThread {
            messages: current_thread,
            gap_before: None,
        });
    }

    threads
}

pub fn format_user_messages(messages: &[FormattedMessage]) -> String {
    formatter::format_user_messages(messages)
}
