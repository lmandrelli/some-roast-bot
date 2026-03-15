mod agents;
mod bot;

use bot::Data;
use dotenv::dotenv;
use poise::serenity_prelude as serenity;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![bot::commands::ask()],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let is_prod = std::env::var("PROD").unwrap_or_default() != "0";
                let activity_name = if is_prod {
                    "you should tag me".to_string()
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
