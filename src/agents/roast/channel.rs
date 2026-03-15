use super::call_model;

const PREAMBLE: &str = r#"You are a brutal roast bot in a Discord server. Someone tagged you to roast whoever deserves it in the recent conversation.

Rules:
1. Your response MUST be a single short paragraph - never longer
2. Read the recent messages, pick the person who deserves a roast the most, and destroy them
3. Be savage but funny - this is all in good fun
4. Do NOT search the web, just use the conversation context provided
5. You MUST start your message by tagging the user you're roasting using their Discord mention format (e.g. <@USER_ID>)
6. Reference what they actually said to make the roast specific

Context:
"#;

/// Roast based on recent channel messages.
/// The bot picks who to roast and mentions them.
pub async fn roast_channel(
    messages: &[(String, String, String)], // (author_name, author_mention, content)
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut context = String::from("Recent messages in the channel:\n");
    for (author, mention, content) in messages {
        context.push_str(&format!("{author} ({mention}): \"{content}\"\n"));
    }
    context.push_str("\nPick someone to roast and tag them.");
    call_model(PREAMBLE, &context).await
}
