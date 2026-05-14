use std::sync::Arc;

use async_trait::async_trait;
use poise::serenity_prelude as serenity;

use crate::agents::llm::LlmService;
use crate::db::MemoryRepository;
use crate::error::BotError;

/// Context passed to every message handler.
pub struct HandlerContext<'a> {
    pub serenity_ctx: &'a serenity::Context,
    pub message: &'a serenity::Message,
    pub mentions_me: bool,
    pub bot_id: serenity::UserId,
    pub memory: Arc<dyn MemoryRepository>,
    pub llm_service: Arc<LlmService>,
}

/// Trait for passive or mention-based message handlers.
///
/// Handlers are checked in ascending `priority` order.
/// `can_handle` is a cheap filter; `handle` does the actual work.
///
/// **Important**: the pipeline runs handlers in priority order and **exactly one**
/// handler is executed per message — the first whose `can_handle` returns `true`.
#[async_trait]
pub trait MessageHandler: Send + Sync {
    fn name(&self) -> &'static str;

    /// Lower values run first.
    fn priority(&self) -> u8;

    /// Quick check without side effects or expensive I/O.
    async fn can_handle(&self, ctx: &HandlerContext<'_>) -> bool;

    /// Execute the handler.
    ///
    /// - `Ok(Some(response))` → the bot will reply with this text
    /// - `Ok(None)`            → the handler handled the message but has no text reply (e.g. social links)
    /// - `Err(e)`              → an error occurred
    async fn handle(&self, ctx: &HandlerContext<'_>) -> Result<Option<String>, BotError>;
}
