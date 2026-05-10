use poise::serenity_prelude::{self as serenity, Timestamp};
use std::time::Duration;

use crate::bot::Error;

const CONVERSATION_GAP_MINUTES: u64 = 5;
const INITIAL_FETCH_LIMIT: usize = 20;
const MAX_FETCH_LIMIT: usize = 50;

#[derive(Debug, Clone)]
pub struct FormattedMessage {
    pub timestamp: Timestamp,
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
        if self.threads.is_empty() {
            return "No recent messages.".to_string();
        }

        let mut output = String::new();

        if self.threads.len() > 1 {
            output.push_str(&format!(
                "Channel activity ({} separate conversations detected):\n\n",
                self.threads.len()
            ));
        } else {
            output.push_str("Recent channel messages (chronological order):\n\n");
        }

        for (thread_idx, thread) in self.threads.iter().enumerate() {
            if self.threads.len() > 1 {
                output.push_str(&format!("--- Conversation {} ---\n", thread_idx + 1));
            }

            if let Some(gap) = thread.gap_before {
                let minutes = gap.as_secs() / 60;
                output.push_str(&format!("(conversation break, {minutes} min gap)\n"));
            }

            for msg in &thread.messages {
                let time_str = msg.timestamp.format("%H:%M");
                let reply_info = match &msg.reply_to {
                    Some(reply_target) => format!(" (replying to {reply_target})"),
                    None => String::new(),
                };
                output.push_str(&format!(
                    "[{time_str}] {}{reply_info}: \"{}\"\n",
                    msg.author_name, msg.content
                ));
            }

            if self.threads.len() > 1 {
                output.push('\n');
            }
        }

        output.push_str(&format!("\nTotal messages shown: {}", self.total_messages));
        if self.can_fetch_more {
            output.push_str(" (can fetch more if needed)");
        }
        output.push('\n');

        output
    }
}

pub async fn fetch_channel_context(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    before_msg_id: serenity::MessageId,
    limit: usize,
    filter_bot: bool,
) -> Result<ChannelContext, Error> {
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
) -> Result<(Vec<FormattedMessage>, ChannelContext), Error> {
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
    if messages.is_empty() {
        return "No messages found.".to_string();
    }

    let mut output = String::new();
    for msg in messages {
        let time_str = msg.timestamp.format("%H:%M");
        let reply_info = match &msg.reply_to {
            Some(reply_target) => format!(" (replying to {reply_target})"),
            None => String::new(),
        };
        output.push_str(&format!(
            "[{time_str}] {}{reply_info}: \"{}\"\n",
            msg.author_name, msg.content
        ));
    }
    output
}
