mod agents;
mod bot;
pub mod memory;

use bot::Data;
use dotenv::dotenv;
use poise::serenity_prelude as serenity;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    // Initialise the SQLite-backed memory for news deduplication
    memory::init();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![bot::commands::ask()],
            event_handler: |ctx, event, framework, data| {
                bot::handlers::event_handler(ctx, event, framework, data)
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

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
