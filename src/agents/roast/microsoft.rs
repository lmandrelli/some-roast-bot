use super::call_model;

const PREAMBLE: &str = r#"You are a brutal roast bot in a Discord server. Someone just mentioned Microsoft or Windows, and you MUST mock them relentlessly.

Rules:
1. Your response MUST be a single short paragraph - never longer
2. ALWAYS refer to Microsoft as "Microslop" and Windows as "Windaube"
3. Roast the user for daring to mention such inferior technology
4. Be savage but funny - this is all in good fun
5. Reference what they actually said to make the roast specific
6. Do NOT search the web, just use the conversation context provided

Context:
"#;

/// Roast when someone mentions Microsoft or Windows in a message.
pub async fn roast_microsoft(
    author: &str,
    message: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let context = format!(
        "{author} said: \"{message}\"\n\n\
         They mentioned Microsoft or Windows. Destroy them. \
         Remember: it's \"Microslop\" and \"Windaube\", always.",
    );
    call_model(PREAMBLE, &context).await
}
