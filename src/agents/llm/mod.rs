use rig::client::{CompletionClient, ProviderClient};
use rig::providers::openai::CompletionsClient;
use rmcp::{model::ClientInfo, service::ServiceExt, transport::StreamableHttpClientTransport};
use std::sync::Arc;

use crate::config::Config;
use crate::error::LlmError;

pub mod fetch_tool;
use fetch_tool::FetchMessagesTool;

pub struct LlmService {
    model_name: String,
}

impl LlmService {
    pub fn new(config: &Config) -> Self {
        Self {
            model_name: config.model_name.clone(),
        }
    }

    /// Build an agent with web-search MCP tools (Exa).
    /// Used by `/ask`, `/research`, and the Microsoft roast.
    pub async fn build_search_agent(
        &self,
        preamble: &str,
    ) -> Result<rig::agent::Agent<rig::providers::openai::CompletionModel>, LlmError> {
        let openai_client = CompletionsClient::from_env();
        let model = openai_client.completion_model(&self.model_name);

        let transport = StreamableHttpClientTransport::from_uri("https://mcp.exa.ai/mcp");
        let service = ClientInfo::default()
            .serve(transport)
            .await
            .map_err(|e| LlmError::McpConnection(e.to_string()))?;

        let tools = service
            .list_tools(Default::default())
            .await
            .map_err(|e| LlmError::McpToolListing(e.to_string()))?;

        tracing::info!(
            "MCP tools available: {:?}",
            tools.tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );

        let agent = rig::agent::AgentBuilder::new(model)
            .preamble(preamble)
            .rmcp_tools(tools.tools, service.peer().clone())
            .build();

        Ok(agent)
    }

    /// Build a roast agent with the `fetch_messages` Discord tool.
    /// Used by channel, reply, user, and truth roasts.
    pub fn build_roast_agent(
        &self,
        preamble: &str,
        ctx: Arc<poise::serenity_prelude::Context>,
        channel_id: poise::serenity_prelude::ChannelId,
    ) -> rig::agent::Agent<rig::providers::openai::CompletionModel> {
        let openai_client = CompletionsClient::from_env();
        let model = openai_client.completion_model(&self.model_name);

        let fetch_tool = FetchMessagesTool { ctx, channel_id };

        rig::agent::AgentBuilder::new(model)
            .preamble(preamble)
            .tool(fetch_tool)
            .build()
    }
}
