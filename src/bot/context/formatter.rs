use crate::bot::context::{ChannelContext, FormattedMessage};

pub fn format_channel_context(ctx: &ChannelContext) -> String {
    if ctx.threads.is_empty() {
        return "No recent messages.".to_string();
    }

    let mut output = String::new();

    if ctx.threads.len() > 1 {
        output.push_str(&format!(
            "Channel activity ({} separate conversations detected):\n\n",
            ctx.threads.len()
        ));
    } else {
        output.push_str("Recent channel messages (chronological order):\n\n");
    }

    for (thread_idx, thread) in ctx.threads.iter().enumerate() {
        if ctx.threads.len() > 1 {
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

        if ctx.threads.len() > 1 {
            output.push('\n');
        }
    }

    output.push_str(&format!("\nTotal messages shown: {}", ctx.total_messages));
    if ctx.can_fetch_more {
        output.push_str(" (can fetch more if needed)");
    }
    output.push('\n');

    output
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
