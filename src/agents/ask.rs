use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai::CompletionsClient;
use rmcp::{model::ClientInfo, service::ServiceExt, transport::StreamableHttpClientTransport};
use std::sync::Arc;
use tokio::sync::RwLock;

pub type SharedAskAgent = Arc<RwLock<Option<AskAgent>>>;

pub struct AskAgent {
    agent: rig::agent::Agent<rig::providers::openai::completion::CompletionModel>,
    // Keeps the MCP transport alive. Dropping this closes the connection.
    _mcp_service: rmcp::service::RunningService<rmcp::service::RoleClient, rmcp::model::ClientInfo>,
}

const PREAMBLE: &str = r#"You are a sarcastic AI assistant that roasts users while answering their questions.

Rules:
1. Search the web using the available tool - 1 search max, 2 only if the first gave nothing useful
2. Roast the user for asking, but still give them the actual answer
3. Your response MUST be a single short paragraph - never longer
4. Focus mostly on the roast, slip the info in naturally

The user asked: "#;

impl AskAgent {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let openai_client = CompletionsClient::from_env();
        let model = openai_client.completion_model("moonshotai/Kimi-K2.5-TEE");

        let transport = StreamableHttpClientTransport::from_uri("https://mcp.exa.ai/mcp");
        let service = ClientInfo::default()
            .serve(transport)
            .await
            .inspect_err(|e| tracing::error!("MCP client error: {:?}", e))?;

        let tools = service.list_tools(Default::default()).await?;
        tracing::info!("Connected to Exa MCP, available tools: {}", tools.tools.len());

        let agent = rig::agent::AgentBuilder::new(model)
            .preamble(PREAMBLE)
            .rmcp_tools(tools.tools, service.peer().clone())
            .build();

        Ok(Self {
            agent,
            _mcp_service: service,
        })
    }

    pub async fn ask(&self, question: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.agent.clone().prompt(question).await?;
        Ok(response)
    }
}
