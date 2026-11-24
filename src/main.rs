mod agents;
mod bot;
pub mod config;
pub mod db;
pub mod error;
pub mod fixers;
pub mod models;

use std::sync::Arc;

use dotenv::dotenv;
use poise::serenity_prelude as serenity;

use agents::llm::LlmService;
use bot::Data;
use config::Config;
use db::sql::SqlMemoryRepository;

#[tokio::main]
async fn main() -> Result<(), BotError> {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    tracing::info!("Starting some-roast-bot v{}", env!("CARGO_PKG_VERSION"));

    let config = Arc::new(Config::from_env()?);
    if std::env::var_os("PROD").is_some() {
        tracing::warn!("PROD is ignored: set STATUS_MESSAGE to change the Discord status");
    }
    let memory = Arc::new(
        SqlMemoryRepository::connect(config.database_url.as_deref(), &config.memory_db_path)
            .await?,
    );
    let llm_service = Arc::new(LlmService::new(&config));
    let pipeline = bot::handlers::build_pipeline();

    let token = config.discord_token.clone();
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                bot::commands::ask(),
                bot::commands::fix(),
                bot::commands::research(),
                bot::commands::stats(),
            ],
            event_handler: |ctx, event, framework, data| {
                Box::pin(bot::handlers::event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                tracing::info!(
                    "Registering commands: {:?}",
                    framework
                        .options()
                        .commands
                        .iter()
                        .map(|c| &c.name)
                        .collect::<Vec<_>>()
                );
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                tracing::info!("Commands registered successfully");

                ctx.set_activity(Some(serenity::ActivityData::custom(
                    config.status_message.clone(),
                )));

                Ok(Data {
                    memory,
                    llm_service,
                    pipeline,
                })
            })
        })
        .build();

    serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?
        .start()
        .await?;

    Ok(())
}

// Re-export BotError at crate root for convenience
pub use error::BotError;
