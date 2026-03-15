use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai::CompletionsClient;
use rmcp::{model::ClientInfo, service::ServiceExt, transport::StreamableHttpClientTransport};

const PREAMBLE: &str = r#"You are Kimi K2.5, a sarcastic AI assistant that roasts users while answering their questions.

Rules:
1. You MUST respond in French as your primary language. Always write in French.
2. Search the web using the available tool - 1 search max, 2 only if the first gave nothing useful
3. Roast the user for asking, but still give them the actual answer
2. Your response MUST never longer than 3 or 4 short sentences.
5. Focus mostly on the roast, slip the info in naturally

The user asked: "#;

pub async fn ask(question: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let model_name = crate::agents::model_name();
    let openai_client = CompletionsClient::from_env();
    let model = openai_client.completion_model(&model_name);

    let transport = StreamableHttpClientTransport::from_uri("https://mcp.exa.ai/mcp");
    let service = ClientInfo::default()
        .serve(transport)
        .await
        .inspect_err(|e| tracing::error!("MCP client error: {:?}", e))?;

    let tools = service.list_tools(Default::default()).await?;
    tracing::info!("MCP tools available: {:?}", tools.tools.iter().map(|t| &t.name).collect::<Vec<_>>());

    let agent = rig::agent::AgentBuilder::new(model)
        .preamble(PREAMBLE)
        .rmcp_tools(tools.tools, service.peer().clone())
        .build();

    tracing::info!("Sending prompt to model...");
    let response = agent.prompt(question).max_turns(2).await
        .inspect_err(|e| tracing::error!("Completion error: {:?}", e))?;
    tracing::info!("Response received: {} chars", response.len());
    Ok(response)
}
