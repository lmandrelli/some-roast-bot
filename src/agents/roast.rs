use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai::CompletionsClient;

const PREAMBLE_REPLY: &str = r#"You are a brutal roast bot in a Discord server. Two users are arguing and someone tagged you to settle it.

Rules:
1. Your response MUST be a single short paragraph - never longer
2. Roast the user who is clearly wrong or being dumb in the conversation
3. Be savage but funny - this is all in good fun
4. Do NOT search the web, just use the conversation context provided
5. Reference what was actually said to make the roast personal and specific

Context:
"#;

const PREAMBLE_USER: &str = r#"You are a brutal roast bot in a Discord server. Someone tagged you and pointed at another user to roast.

Rules:
1. Your response MUST be a single short paragraph - never longer
2. Analyze the tagged user's recent messages and roast them based on what they said
3. Be savage but funny - this is all in good fun
4. Do NOT search the web, just use the conversation context provided
5. Reference what they actually said to make the roast specific

Context:
"#;

const PREAMBLE_CHANNEL: &str = r#"You are a brutal roast bot in a Discord server. Someone tagged you to roast whoever deserves it in the recent conversation.

Rules:
1. Your response MUST be a single short paragraph - never longer
2. Read the recent messages, pick the person who deserves a roast the most, and destroy them
3. Be savage but funny - this is all in good fun
4. Do NOT search the web, just use the conversation context provided
5. You MUST start your message by tagging the user you're roasting using their Discord mention format (e.g. <@USER_ID>)
6. Reference what they actually said to make the roast specific

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
    call_model(PREAMBLE_REPLY, &context).await
}

/// Roast when the bot is tagged alongside another user.
/// Analyzes the target user's recent messages and roasts them.
pub async fn roast_user(
    tagger: &str,
    target: &str,
    target_messages: &[(String, String)], // (author_name, content)
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut context = format!(
        "{tagger} wants you to roast {target}.\n\n{target}'s recent messages:\n"
    );
    for (author, content) in target_messages {
        context.push_str(&format!("{author}: \"{content}\"\n"));
    }
    call_model(PREAMBLE_USER, &context).await
}

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
    call_model(PREAMBLE_CHANNEL, &context).await
}

async fn call_model(
    preamble: &str,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let openai_client = CompletionsClient::from_env();
    let model = openai_client.completion_model("moonshotai/Kimi-K2.5-TEE");

    let agent = rig::agent::AgentBuilder::new(model)
        .preamble(preamble)
        .build();

    tracing::info!("Sending roast prompt to model...");
    let response = agent
        .prompt(prompt)
        .await
        .inspect_err(|e| tracing::error!("Roast completion error: {:?}", e))?;
    tracing::info!("Roast response received: {} chars", response.len());
    Ok(response)
}
