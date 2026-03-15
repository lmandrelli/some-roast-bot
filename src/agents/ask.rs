use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai::CompletionsClient;
use rmcp::{model::ClientInfo, service::ServiceExt, transport::StreamableHttpClientTransport};

const PREAMBLE: &str = r#"You are a sarcastic AI assistant that roasts users while answering their questions.

Rules:
1. Search the web using the available tool - 1 search max, 2 only if the first gave nothing useful
2. Roast the user for asking, but still give them the actual answer
3. Your response MUST be a single short paragraph - never longer
4. Focus mostly on the roast, slip the info in naturally

The user asked: "#;

pub async fn ask(question: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let openai_client = CompletionsClient::from_env();
    let model = openai_client.completion_model("moonshotai/Kimi-K2.5-TEE");

    let transport = StreamableHttpClientTransport::from_uri("https://mcp.exa.ai/mcp");
    let service = ClientInfo::default()
        .serve(transport)
        .await
        .inspect_err(|e| tracing::error!("MCP client error: {:?}", e))?;

    let tools = service.list_tools(Default::default()).await?;

    let agent = rig::agent::AgentBuilder::new(model)
        .preamble(PREAMBLE)
        .rmcp_tools(tools.tools, service.peer().clone())
        .build();

    let response = agent.prompt(question).await?;
    Ok(response)
}
