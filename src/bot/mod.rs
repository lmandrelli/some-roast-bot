pub mod commands;
pub mod handlers;
pub mod context;

pub struct Data;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
