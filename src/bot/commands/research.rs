use crate::bot::{Context, Error};

/// Ask for a detailed, researched answer
#[poise::command(slash_command)]
pub async fn research(
    ctx: Context<'_>,
    #[description = "Your question"] question: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    match crate::agents::research(&ctx.data().llm_service, &question).await {
        Ok(response) => {
            ctx.say(response).await?;
        }
        Err(e) => {
            tracing::error!("Research command failed: {:?}", e);
            let error_response = crate::error::discord_error_response(&e);
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
