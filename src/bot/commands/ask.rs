use crate::bot::{Context, Error};

/// Ask the roast bot a question
#[poise::command(slash_command)]
pub async fn ask(
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
