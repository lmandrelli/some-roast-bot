mod agent;

use agent::{RoastAgent, SharedAgent};
use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use std::sync::Arc;
use tokio::sync::RwLock;

struct Data {
    agent: SharedAgent,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command)]
async fn ask(
    ctx: Context<'_>,
    #[description = "Your question"] question: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let agent_read = ctx.data().agent.read().await;
    let agent = agent_read.as_ref().ok_or("Agent not initialized")?;

    match agent.ask(&question).await {
        Ok(response) => {
            ctx.say(response).await?;
        }
        Err(e) => {
            ctx.say(format!("Error: {}", e)).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let shared_agent: SharedAgent = Arc::new(RwLock::new(None));

    let _data = Data {
        agent: shared_agent.clone(),
    };

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![ask()],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            let agent_clone = shared_agent.clone();
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let roast_agent = match RoastAgent::new().await {
                    Ok(a) => {
                        tracing::info!("Roast agent initialized successfully");
                        Some(a)
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize roast agent: {}", e);
                        None
                    }
                };

                let mut guard = agent_clone.write().await;
                *guard = roast_agent;
                drop(guard);

                Ok(Data { agent: agent_clone })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
