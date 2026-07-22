use crate::config::Config;
use crate::error::LlmError;
use rig::client::{CompletionClient, ProviderClient};
use rig::providers::openai::CompletionsClient;
use rmcp::{model::ClientInfo, service::ServiceExt, transport::StreamableHttpClientTransport};

/// Opaque handle to the running MCP session.
/// Callers must hold this value alive for as long as the agent's tools
/// might be invoked — dropping it closes the transport.
pub type McpSession = rmcp::service::RunningService<rmcp::RoleClient, ClientInfo>;

pub struct LlmService {
    model_name: String,
    vision_model_name: String,
    magic_word: String,
}

impl LlmService {
    pub fn new(config: &Config) -> Self {
        Self {
            model_name: config.model_name.clone(),
            vision_model_name: config.vision_model_name.clone(),
            magic_word: config.magic_word.clone(),
        }
    }

    /// Build the isolated vision agent. It deliberately has no MCP tools.
    pub fn build_vision_agent(
        &self,
        preamble: &str,
    ) -> rig::agent::Agent<rig::providers::openai::CompletionModel> {
        let client = CompletionsClient::from_env();
        rig::agent::AgentBuilder::new(client.completion_model(&self.vision_model_name))
            .preamble(preamble)
            .build()
    }

    /// Magic word (already lowercased) that activates the easter-egg
    /// override in every agent.
    pub fn magic_word(&self) -> &str {
        &self.magic_word
    }

    /// Build an agent with Exa MCP web-search tools.
    /// All roast agents get Exa access; behaviour is controlled via the preamble.
    ///
    /// Returns both the agent **and** the MCP session handle.  The session
    /// **must** be kept alive (not dropped) for the entire duration of any
    /// prompt call, otherwise tool calls will fail with "Transport closed".
    pub async fn build_agent(
        &self,
        preamble: &str,
    ) -> Result<
        (
            rig::agent::Agent<rig::providers::openai::CompletionModel>,
            McpSession,
        ),
        LlmError,
    > {
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

        Ok((agent, service))
    }
}
