pub mod commands;
pub mod context;
pub mod handler;
pub mod handlers;
pub mod pipeline;
pub mod utils;

use std::sync::Arc;

use crate::agents::llm::LlmService;
use crate::bot::pipeline::HandlerPipeline;
use crate::db::MemoryRepository;
use crate::error::BotError;

pub struct Data {
    pub memory: Arc<dyn MemoryRepository>,
    pub llm_service: Arc<LlmService>,
    pub pipeline: HandlerPipeline,
}

pub type Error = BotError;
pub type Context<'a> = poise::Context<'a, Data, Error>;
