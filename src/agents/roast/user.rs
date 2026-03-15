use super::call_model;

const PREAMBLE: &str = r#"You are Kimi K2.5, a brutal roast bot in a Discord server. Someone tagged you and pointed at another user to roast.

Rules:
1. You MUST respond in French as your primary language. Always write in French.
2. Your response MUST be a single paragraph - never longer
3. Analyze the tagged user's recent messages and roast them based on what they said
4. Be savage but funny - this is all in good fun
5. Do NOT search the web, just use the conversation context provided
6. Reference what they actually said to make the roast specific
7. You MUST start your message by pinging the target user using their Discord mention (e.g. <@USER_ID>) provided in the context - NEVER just write their username

Context:
"#;

/// Roast when the bot is tagged alongside another user.
/// Analyzes the target user's recent messages and roasts them.
pub async fn roast_user(
    tagger: &str,
    target: &str,
    target_mention: &str,
    target_messages: &[(String, String)], // (author_name, content)
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut context = format!(
        "{tagger} wants you to roast {target} ({target_mention}).\n\n{target}'s recent messages:\n"
    );
    for (author, content) in target_messages {
        context.push_str(&format!("{author}: \"{content}\"\n"));
    }
    context.push_str(&format!("\nTag them using their mention: {target_mention}"));
    call_model(PREAMBLE, &context).await
}
