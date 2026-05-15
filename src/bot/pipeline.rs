use crate::bot::handler::MessageHandler;
use crate::error::BotError;

/// A pipeline of [`MessageHandler`]s that runs them in priority order
/// and executes **exactly one** handler — the first whose `can_handle`
/// returns `true`.
pub struct HandlerPipeline {
    handlers: Vec<Box<dyn MessageHandler>>,
}

impl HandlerPipeline {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn register(&mut self, handler: Box<dyn MessageHandler>) {
        self.handlers.push(handler);
    }

    /// Check whether any handler in the pipeline is interested in this message.
    /// This is a cheap filter (no side-effects, no I/O) used to decide whether
    /// to show the "typing" indicator.
    pub async fn has_match(&self, ctx: &crate::bot::handler::HandlerContext<'_>) -> bool {
        let mut handlers: Vec<_> = self.handlers.iter().collect();
        handlers.sort_by_key(|h| h.priority());

        for handler in handlers {
            if handler.can_handle(ctx).await {
                return true;
            }
        }

        false
    }

    pub async fn run(
        &self,
        ctx: &crate::bot::handler::HandlerContext<'_>,
    ) -> Result<Option<String>, BotError> {
        let mut handlers: Vec<_> = self.handlers.iter().collect();
        handlers.sort_by_key(|h| h.priority());

        for handler in handlers {
            if handler.can_handle(ctx).await {
                tracing::info!(
                    "Handler '{}' matched message from {} in channel {} — executing",
                    handler.name(),
                    ctx.message.author.name,
                    ctx.message.channel_id
                );

                // Execute the highest-priority matching handler and return
                // immediately. Exactly ONE handler runs per message.
                return handler.handle(ctx).await;
            }
        }

        Ok(None)
    }
}
