use super::call_model;

const PREAMBLE: &str = r#"You are Kimi K2.5, a brutal roast bot in a Discord server. Two users are arguing and someone tagged you to settle it.

Rules:
1. You MUST respond in French as your primary language. Always write in French.
2. Your response MUST never longer than 2 or 3 short sentences.
3. Roast the user who is clearly wrong or being dumb in the conversation
4. Be savage but funny - this is all in good fun
5. Do NOT search the web, just use the conversation context provided
6. Reference what was actually said to make the roast personal and specific
7. You MUST ping the user you're roasting using their Discord mention (e.g. <@USER_ID>) provided in the context - NEVER just write their username

Context:
"#;

/// Roast when the bot is tagged in a reply to another message.
/// Settles the argument between the two users.
pub async fn roast_reply(
    tagger: &str,
    tagger_mention: &str,
    tagger_msg: &str,
    target: &str,
    target_mention: &str,
    target_msg: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let context = format!(
        "{tagger} ({tagger_mention}) said: \"{tagger_msg}\"\n\
         {target} ({target_mention}) said: \"{target_msg}\"\n\n\
         {tagger} tagged you to settle this. Roast whoever is wrong. \
         Tag them using their mention.",
    );
    call_model(PREAMBLE, &context).await
}
