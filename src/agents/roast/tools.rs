use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::bot::context;

#[derive(Debug, thiserror::Error)]
#[error("Failed to fetch messages: {0}")]
pub struct FetchError(String);

#[derive(Deserialize)]
pub struct FetchMessagesArgs {
    pub before_message_id: String,
    pub limit: usize,
}

pub struct FetchMessagesTool {
    pub ctx: Arc<poise::serenity_prelude::Context>,
    pub channel_id: poise::serenity_prelude::ChannelId,
}

impl Tool for FetchMessagesTool {
    const NAME: &'static str = "fetch_messages";

    type Error = FetchError;
    type Args = FetchMessagesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "fetch_messages".to_string(),
            description: "Fetch recent messages from the Discord channel. Use this when you need more context about the conversation. Messages are returned in chronological order with timestamps and conversation threading information.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "before_message_id": {
                        "type": "string",
                        "description": "The message ID to fetch before (get messages older than this). Use the oldest message ID from the current context if available."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of messages to fetch (1-50, default 20)"
                    }
                },
                "required": ["before_message_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let limit = args.limit.min(50).max(1);
        let before_id = args
            .before_message_id
            .parse::<u64>()
            .map_err(|e| FetchError(format!("Invalid message ID: {e}")))?;

        let channel_ctx = context::fetch_channel_context(
            &self.ctx,
            self.channel_id,
            poise::serenity_prelude::MessageId::new(before_id),
            limit,
            true,
        )
        .await
        .map_err(|e| FetchError(e.to_string()))?;

        Ok(channel_ctx.to_string())
    }
}
