use crate::bot::Data;
use crate::bot::Error;
use crate::memory;
use poise::serenity_prelude as serenity;

#[poise::command(slash_command, prefix_command)]
pub async fn stats(ctx: poise::Context<'_, Data, Error>) -> Result<(), Error> {
    let (title, content) = format_stats();

    let embed = serenity::CreateEmbed::new()
        .title(&title)
        .description(content)
        .color(serenity::Colour::from_rgb(255, 0, 0));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

fn format_stats() -> (String, String) {
    let roast_types = [
        ("channel", "Roast par mention de Some(roast)"),
        ("user", "Roast par mention d'un utilisateur"),
        ("reply", "Roast par réponse à un message"),
        ("truth", "is this true?"),
        ("microsoft", "Microslop Windaube Copyslop Premium"),
        ("quoi", "Quoi ? -feur (ça fait coiffeur mdr)"),
    ];

    let mut content = String::new();

    for (roast_type, title) in roast_types.iter() {
        let count = memory::get_roast_count(roast_type);
        content.push_str(&format!("### {title}\n"));

        if count > 0 {
            content.push_str(&format!("**{}**\n", count));

            if *roast_type != "microsoft" {
                let triggerers = memory::get_top_triggerers(roast_type, 3);
                if !triggerers.is_empty() {
                    content.push_str("**Top déclencheurs**\n");
                    for (i, (user_id, cnt)) in triggerers.iter().enumerate() {
                        if i > 0 {
                            content.push_str(", ");
                        }
                        content.push_str(&format!("<@{user_id}> ({cnt})"));
                    }
                    content.push_str("\n");
                }
            }

            let targets = memory::get_top_targets(roast_type, 3);
            if !targets.is_empty() {
                content.push_str("**Top victimes**\n");
                for (i, (user_id, cnt)) in targets.iter().enumerate() {
                    if i > 0 {
                        content.push_str(", ");
                    }
                    content.push_str(&format!("<@{user_id}> ({cnt})"));
                }
                content.push_str("\n");
            }
        } else {
            content.push_str("**0**\n");
        }

        if *roast_type != "quoi" {
            content.push_str("---\n");
        }
    }

    let content = content.trim_end_matches("---\n").to_string();

    ("📊 Some(stats):".to_string(), content)
}