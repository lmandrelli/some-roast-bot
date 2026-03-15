mod agents;
mod bot;

use agents::{AskAgent, SharedAskAgent};
use bot::Data;
use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let shared_agent: SharedAskAgent = Arc::new(RwLock::new(None));

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![bot::commands::ask()],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            let agent = shared_agent.clone();
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let ask_agent = match AskAgent::new().await {
                    Ok(a) => {
                        tracing::info!("Ask agent initialized successfully");
                        Some(a)
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize ask agent: {}", e);
                        None
                    }
                };

                *agent.write().await = ask_agent;

                Ok(Data { agent })
            })
        })
        .build();

    serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .unwrap()
        .start()
        .await
        .unwrap();
}
