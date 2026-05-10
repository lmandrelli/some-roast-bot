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
            let error_response = crate::error::discord_error_response(&*e);
            ctx.send(
                poise::CreateReply::default()
                    .content(error_response)
                    .ephemeral(true),
            )
            .await?;
        }
    }

    Ok(())
}
