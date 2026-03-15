use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai::CompletionsClient;
use rmcp::{model::ClientInfo, service::ServiceExt, transport::StreamableHttpClientTransport};

const PREAMBLE: &str = r#"You are Kimi K2.5, a brutal roast bot in a Discord server. Someone just mentioned Microsoft or Windows, and you MUST mock them relentlessly.

STEP 1 — CHECK ALREADY USED TOPICS:
Read the "Already Used Topics" list below BEFORE searching. You MUST NOT reuse any of them.

STEP 2 — SEARCH FOR FRESH NEWS:
Search the web for the latest Microsoft or Windows fails, bugs, controversies, or dumb decisions.
Prefer community sources like Reddit: r/MicroSlop (https://www.reddit.com/r/MicroSlop/), r/windows, r/microsoft, r/sysadmin.
Pick a topic that is NOT in the already used list.

STEP 3 — WRITE YOUR ROAST:
Rules:
1. You MUST respond in French as your primary language. Always write in French.
2. Your response MUST be a really short sentence saying "On dit Microslop ici" or "C'est Windaube pas Windows !", or similar; then follow up with 2 or 3 short sentences about a way microsoft or windows has done something dumb or annoying.
3. ALWAYS refer to Microsoft as "Microslop" and Windows as "Windaube". You're actually roasting Microsoft and Windows, not the user here.
4. Be savage but funny - this is all in good fun
5. Reference what they actually said to make the roast specific
6. You MUST start your message by pinging the user using their Discord mention (e.g. <@USER_ID>) provided in the context - NEVER just write their username
7. At the VERY END of your message, on a new line, write exactly: [TOPIC: short description of the news you used]

"#;

/// Roast when someone mentions Microsoft or Windows in a message.
/// Uses Exa web search to find latest Microsoft news and SQLite memory
/// to avoid repeating the same topics.
pub async fn roast_microsoft(
    author: &str,
    author_mention: &str,
    message: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Fetch previously used topics from memory
    let past_topics = crate::memory::recent_topics(20);
    let topics_section = if past_topics.is_empty() {
        "Already Used Topics: (none yet)\n".to_string()
    } else {
        let list = past_topics
            .iter()
            .enumerate()
            .map(|(i, t)| format!("  {}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n");
        format!("Already Used Topics (DO NOT reuse these):\n{list}\n")
    };

    let full_preamble = format!("{PREAMBLE}\n{topics_section}\n---\nContext:\n");

    let context = format!(
        "{author} ({author_mention}) said: \"{message}\"\n\n\
         They mentioned Microsoft or Windows. Destroy them. \
         Remember: it's \"Microslop\" and \"Windaube\", always. \
         Tag them using their mention: {author_mention}",
    );

    // Build agent with MCP tools (Exa search)
    let model_name = crate::agents::model_name();
    let openai_client = CompletionsClient::from_env();
    let model = openai_client.completion_model(&model_name);

    let transport = StreamableHttpClientTransport::from_uri("https://mcp.exa.ai/mcp");
    let service = ClientInfo::default()
        .serve(transport)
        .await
        .inspect_err(|e| tracing::error!("MCP client error: {:?}", e))?;

    let tools = service.list_tools(Default::default()).await?;
    tracing::info!(
        "MCP tools available for microsoft roast: {:?}",
        tools.tools.iter().map(|t| &t.name).collect::<Vec<_>>()
    );

    let agent = rig::agent::AgentBuilder::new(model)
        .preamble(&full_preamble)
        .rmcp_tools(tools.tools, service.peer().clone())
        .build();

    tracing::info!("Sending microsoft roast prompt to model ({model_name})...");
    let response = agent
        .prompt(&context)
        .max_turns(5)
        .await
        .inspect_err(|e| tracing::error!("Microsoft roast completion error: {:?}", e))?;
    tracing::info!("Microsoft roast response received: {} chars", response.len());

    // Extract and store the topic from the [TOPIC: ...] tag
    let (clean_response, topic) = extract_topic(&response);
    if let Some(topic) = topic {
        tracing::info!("Storing microsoft news topic: {topic}");
        crate::memory::remember_topic(&topic);
    }

    Ok(clean_response)
}

/// Extract `[TOPIC: ...]` from the end of the response.
/// Returns (cleaned response, optional topic).
fn extract_topic(response: &str) -> (String, Option<String>) {
    if let Some(start) = response.rfind("[TOPIC:") {
        if let Some(end) = response[start..].find(']') {
            let topic = response[start + 7..start + end].trim().to_string();
            let clean = response[..start].trim().to_string();
            return (clean, Some(topic));
        }
    }
    (response.to_string(), None)
}
