pub mod commands;

use crate::agents::SharedAskAgent;

pub struct Data {
    pub agent: SharedAskAgent,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
