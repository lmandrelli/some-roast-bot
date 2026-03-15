use crate::bot::{Context, Error};

/// Ask the roast bot a question
#[poise::command(slash_command)]
pub async fn ask(
    ctx: Context<'_>,
    #[description = "Your question"] question: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    match crate::agents::ask(&question).await {
        Ok(response) => {
            ctx.say(response).await?;
        }
        Err(e) => {
            tracing::error!("Ask command failed: {:?}", e);
            ctx.send(
                poise::CreateReply::default()
                    .content(format!("**Error:** {}", e))
                    .ephemeral(true),
            )
            .await?;
        }
    }

    Ok(())
}
