//! Prompt evaluation on made-up conversations, against a real model.
//! Ignored by default: needs DISCORD_TOKEN (any value), OPENAI_API_KEY,
//! OPENAI_BASE_URL and MODEL_NAME, plus network access to the Exa MCP.
//!
//! cargo test eval -- --ignored --nocapture --test-threads=1

use poise::serenity_prelude as serenity;

use crate::agents::llm::LlmService;
use crate::bot::context::{AuthorIdentity, ChannelContext, TranscriptMessage};
use crate::config::Config;

const BOT: u64 = 999;

fn who(id: u64, name: &str) -> AuthorIdentity {
    AuthorIdentity {
        id: serenity::UserId::new(id),
        name: name.into(),
        is_bot: id == BOT,
        is_self: id == BOT,
    }
}

struct Line {
    minutes_ago: i64,
    author: AuthorIdentity,
    text: String,
    reply_to: Option<usize>,
}

fn line(minutes_ago: i64, author: &AuthorIdentity, text: &str) -> Line {
    Line {
        minutes_ago,
        author: author.clone(),
        text: text.into(),
        reply_to: None,
    }
}

/// The last line is the trigger.
fn context(lines: Vec<Line>) -> ChannelContext {
    let now = chrono::Utc::now().timestamp();
    let last = lines.len() - 1;
    let targets: Vec<usize> = lines.iter().filter_map(|l| l.reply_to).collect();
    let messages = lines
        .iter()
        .enumerate()
        .map(|(i, l)| TranscriptMessage {
            id: serenity::MessageId::new(10_000 + i as u64),
            timestamp: serenity::Timestamp::from_unix_timestamp(now - l.minutes_ago * 60).unwrap(),
            author: l.author.clone(),
            content: l.text.clone(),
            is_trigger: i == last,
            is_reply_target: i != last && targets.contains(&i) && lines[last].reply_to == Some(i),
            reply_to: l.reply_to.map(|t| {
                (
                    serenity::MessageId::new(10_000 + t as u64),
                    Some(lines[t].author.clone()),
                )
            }),
            attachments: vec![],
            stickers: vec![],
            embeds: vec![],
            visuals: vec![],
        })
        .collect();
    ChannelContext {
        guild_name: "Les Potes".into(),
        channel_name: "général".into(),
        channel_description: None,
        messages,
    }
}

fn mention() -> String {
    format!("@Some(roast) (<@{BOT}>)")
}

#[tokio::test]
#[ignore]
async fn eval() {
    let config = Config::from_env().expect("env");
    let llm = LlmService::new(&config);
    let bapt = who(1, "Bapt");
    let kevin = who(2, "Kevin");
    let thomas = who(3, "Thomas");
    let julie = who(4, "Julie");
    let max = who(5, "Max");
    let lea = who(6, "Léa");
    let bot = who(BOT, "Some(roast)");

    let mut reply_trigger = line(0, &bapt, &format!("{} .", mention()));
    reply_trigger.reply_to = Some(0);
    let reply = context(vec![
        line(
            42,
            &kevin,
            "franchement le Port-Salut c'est le meilleur fromage de France, je débats pas",
        ),
        line(40, &julie, "qui est chaud pour une ranked ce soir ?"),
        line(39, &max, "moi après 21h"),
        line(38, &bapt, "go, je ramène les chips"),
        line(30, &julie, "Max tu joues support cette fois stp"),
        line(29, &max, "jamais de la vie"),
        line(
            12,
            &thomas,
            "vous avez vu le prix des places pour le concert ?",
        ),
        line(11, &julie, "150 balles en fosse, ils sont fous"),
        line(10, &max, "je regarderai sur YouTube"),
        reply_trigger,
    ]);

    let mut weeb_trigger = line(0, &bapt, &format!("{} roast ce weeb", mention()));
    weeb_trigger.reply_to = Some(1);
    let weeb = context(vec![
        line(6, &julie, "vous faites quoi ce soir ?"),
        line(
            5,
            &thomas,
            "je commence celui-là, 170 épisodes ça passe crème https://myanimelist.net/anime/34572",
        ),
        line(4, &max, "courage"),
        weeb_trigger,
    ]);

    let general = context(vec![
        line(
            8,
            &julie,
            "j'ai encore raté mon train, 3e fois cette semaine",
        ),
        line(7, &max, "t'as pensé à partir plus tôt ?"),
        line(7, &julie, "non je suis une artiste, je vis à l'instinct"),
        line(
            6,
            &bot,
            "L'instinct de Julie, c'est la SNCF version humaine : toujours en retard, jamais remboursée.",
        ),
        line(2, &julie, "et là le contrôleur m'a mis une amende en plus"),
        line(0, &max, &format!("{} ton avis ?", mention())),
    ]);

    let target = context(vec![
        line(
            20,
            &lea,
            "j'ai acheté un NFT de singe hier, c'est un investissement long terme",
        ),
        line(19, &max, "il vaut combien aujourd'hui ?"),
        line(18, &lea, "3 euros mais ça va remonter"),
        line(15, &kevin, "quelqu'un a un chargeur USB-C ?"),
        line(0, &bapt, &format!("{} détruis @Léa (<@6>)", mention())),
    ]);

    let truth = context(vec![
        line(
            3,
            &max,
            "les huîtres sont encore vivantes quand on les mange, c'est pour ça qu'elles sont fraîches",
        ),
        line(2, &kevin, "n'importe quoi"),
        line(0, &julie, "is this true?"),
    ]);

    for (name, ctx) in [
        ("reply", &reply),
        ("weeb", &weeb),
        ("general", &general),
        ("target", &target),
    ] {
        for run in 1..=2 {
            let out = crate::agents::roast_channel(&llm, ctx).await;
            println!(
                "\n=== {name} #{run}\n{}",
                out.map(|o| o.response)
                    .unwrap_or_else(|e| format!("ERROR {e}"))
            );
        }
    }
    for run in 1..=2 {
        let out = crate::agents::roast_truth(&llm, &truth).await;
        println!(
            "\n=== truth #{run}\n{}",
            out.map(|o| o.response)
                .unwrap_or_else(|e| format!("ERROR {e}"))
        );
    }
    let ask_ctx = context(vec![
        line(5, &max, "ce soir je fais une raclette"),
        line(4, &julie, "en novembre ? t'es sérieux"),
        line(3, &max, "y a pas de saison pour le fromage"),
    ]);
    for run in 1..=2 {
        let out = crate::agents::ask(
            &llm,
            &max,
            "combien de calories dans une raclette ?",
            &ask_ctx,
        )
        .await;
        println!(
            "\n=== ask #{run}\n{}",
            out.unwrap_or_else(|e| format!("ERROR {e}"))
        );
    }
}
