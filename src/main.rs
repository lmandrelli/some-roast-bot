mod agents;
mod bot;
pub mod error;
pub mod fixers;
pub mod memory;
pub mod models;

use bot::Data;
use dotenv::dotenv;
use poise::serenity_prelude as serenity;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    tracing::info!("Starting some-roast-bot v{}", env!("CARGO_PKG_VERSION"));

    // Initialise the SQLite-backed memory for news deduplication
    memory::init();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
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
                bot::handlers::event_handler(ctx, event, framework, data)
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

                let is_prod = std::env::var("PROD").unwrap_or_default() != "0";
                let activity_name = if is_prod {
                    "don't try to talk about Microsoft".to_string()
                } else {
                    format!("running v{}", env!("CARGO_PKG_VERSION"))
                };

                ctx.set_activity(Some(serenity::ActivityData::custom(activity_name)));

                Ok(Data)
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
