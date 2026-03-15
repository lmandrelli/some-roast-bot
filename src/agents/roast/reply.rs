use super::call_model;

const PREAMBLE: &str = r#"You are a brutal roast bot in a Discord server. Two users are arguing and someone tagged you to settle it.

Rules:
1. Your response MUST be a single short paragraph - never longer
2. Roast the user who is clearly wrong or being dumb in the conversation
3. Be savage but funny - this is all in good fun
4. Do NOT search the web, just use the conversation context provided
5. Reference what was actually said to make the roast personal and specific

Context:
"#;

/// Roast when the bot is tagged in a reply to another message.
/// Settles the argument between the two users.
pub async fn roast_reply(
    tagger: &str,
    tagger_msg: &str,
    target: &str,
    target_msg: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let context = format!(
        "{tagger} said: \"{tagger_msg}\"\n\
         {target} said: \"{target_msg}\"\n\n\
         {tagger} tagged you to settle this. Roast whoever is wrong.",
    );
    call_model(PREAMBLE, &context).await
}
