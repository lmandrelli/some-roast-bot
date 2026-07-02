use crate::config::Config;
use crate::error::LlmError;
use rig::client::{CompletionClient, ProviderClient};
use rig::providers::openai::CompletionsClient;
use rmcp::{model::ClientInfo, service::ServiceExt, transport::StreamableHttpClientTransport};

pub struct LlmService {
    model_name: String,
    magic_word: String,
}

impl LlmService {
    pub fn new(config: &Config) -> Self {
        Self {
            model_name: config.model_name.clone(),
            magic_word: config.magic_word.clone(),
        }
    }

    /// Magic word (already lowercased) that activates the easter-egg
    /// override in every agent.
    pub fn magic_word(&self) -> &str {
        &self.magic_word
    }

    /// Build an agent with Exa MCP web-search tools.
    /// All roast agents get Exa access; behaviour is controlled via the preamble.
    pub async fn build_agent(
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
}
