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
    model_providers: Vec<String>,
    magic_word: String,
}

impl LlmService {
    pub fn new(config: &Config) -> Self {
        Self {
            model_name: config.model_name.clone(),
            model_providers: config.model_providers.clone(),
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

        let mut builder = rig::agent::AgentBuilder::new(model)
            .preamble(preamble)
            .rmcp_tools(tools.tools, service.peer().clone());
        if let Some(routing) = provider_routing(&self.model_providers) {
            builder = builder.additional_params(routing);
        }
        let agent = builder.build();

        Ok((agent, service))
    }
}

/// OpenRouter `provider` routing: only these endpoints, in this order.
fn provider_routing(providers: &[String]) -> Option<serde_json::Value> {
    (!providers.is_empty()).then(|| {
        serde_json::json!({
            "provider": { "order": providers, "allow_fallbacks": false }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::provider_routing;

    #[test]
    fn routes_only_listed_providers_in_order() {
        assert_eq!(provider_routing(&[]), None);
        assert_eq!(
            provider_routing(&["z-ai/fp8".into(), "fireworks".into()]).unwrap(),
            serde_json::json!({
                "provider": { "order": ["z-ai/fp8", "fireworks"], "allow_fallbacks": false }
            })
        );
    }
}
