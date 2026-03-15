use rig::agent::AgentBuilder;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Prompt;
use rig::providers::openai::CompletionsClient;
use rmcp::{
    model::ClientInfo,
    service::{RunningService, RoleClient, ServiceExt},
    transport::StreamableHttpClientTransport,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RoastAgent {
    agent: rig::agent::Agent<rig::providers::openai::completion::CompletionModel>,
    // Keeps the MCP transport background task alive. Dropping RunningService
    // triggers its DropGuard which cancels the task, closing the connection.
    _mcp_service: RunningService<RoleClient, ClientInfo>,
}

impl RoastAgent {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let openai_client = CompletionsClient::from_env();
        let model = openai_client.completion_model("moonshotai/Kimi-K2.5-TEE");

        let transport = StreamableHttpClientTransport::from_uri("https://mcp.exa.ai/mcp");

        let client_info = ClientInfo::default();

        let service = client_info.serve(transport).await.inspect_err(|e| {
            tracing::error!("MCP client error: {:?}", e);
        })?;

        let tools = service.list_tools(Default::default()).await?;
        tracing::info!(
            "Connected to Exa MCP, available tools: {:?}",
            tools.tools.len()
        );

        let preamble = r#"You are a sarcastic AI assistant that roasts users. Your job is to answer their questions while mocking them.

When given a question:
1. Search the web using the available search tool - do ONLY 1 search, MAXIMUM 2 if the first didn't give useful results
2. Use the information found to give a witty, sarcastic answer that roasts the user for asking
3. Be clever and funny, but not overly cruel

Keep your response concise and entertaining. The user asked: "#;

        let peer = service.peer().clone();

        let agent = AgentBuilder::new(model)
            .preamble(preamble)
            .rmcp_tools(tools.tools, peer)
            .build();

        Ok(Self { agent, _mcp_service: service })
    }

    pub async fn ask(
        &self,
        question: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.agent.clone().prompt(question).await?;
        Ok(response)
    }
}

pub type SharedAgent = Arc<RwLock<Option<RoastAgent>>>;
