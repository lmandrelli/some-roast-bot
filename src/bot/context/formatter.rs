use crate::bot::context::ChannelContext;

pub fn format_channel_context(ctx: &ChannelContext) -> String {
    let mut output = String::new();
    output.push_str("Recent conversation (up to 15 messages, chronological order):\n\n");

    if let Some(trigger) = &ctx.trigger {
        let time_str = trigger.timestamp.format("%H:%M");
        output.push_str(&format!(
            "[TRIGGER — {time_str}] @{} (<@{}>): \"{}\"\n",
            trigger.author_name, trigger.author_mention_id, trigger.content
        ));
    }

    for msg in &ctx.messages {
        let time_str = msg.timestamp.format("%H:%M");
        output.push_str(&format!(
            "[{time_str}] @{} (<@{}>): \"{}\"\n",
            msg.author_name, msg.author_mention_id, msg.content
        ));
    }

    output
}
