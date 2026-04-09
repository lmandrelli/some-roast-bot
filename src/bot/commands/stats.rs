use crate::bot::Data;
use crate::bot::Error;
use crate::memory;
use poise::serenity_prelude as serenity;

#[poise::command(slash_command, prefix_command)]
pub async fn stats(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    let (microsoft_count, quoi_feur_count) = memory::get_stats();
    let version = env!("CARGO_PKG_VERSION");

    let embed = serenity::CreateEmbed::new()
        .title("Bot Statistics")
        .description(format!(
            "Number of microsoft roasting : {}\nNumber of -feur : {}\n---\nRunning v{}",
            microsoft_count, quoi_feur_count, version
        ))
        .color(serenity::Colour::from_rgb(255, 0, 0));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
