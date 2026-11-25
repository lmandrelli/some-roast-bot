//! Prompt evaluation on made-up conversations, against a real model.
//! Ignored by default: needs DISCORD_TOKEN (any value), OPENAI_API_KEY,
//! OPENAI_BASE_URL and MODEL_NAME, plus network access to the Exa MCP and
//! to the linked pages.
//!
//! cargo test eval -- --ignored --nocapture --test-threads=1
//! EVAL_CASE=dot_old,forward restricts the run to some cases.
//!
//! `replay` runs a real conversation instead: REPLAY_JSON is a file with
//! the messages returned by GET /channels/{id}/messages (newest first, as
//! Discord sends them), REPLAY_TRIGGER the id of the trigger message and
//! REPLAY_SELF_ID the bot's user id. Keep that file out of the repository.

use poise::serenity_prelude as serenity;

use crate::agents::llm::LlmService;
use crate::bot::context::{AuthorIdentity, ChannelContext, EmbedData, TranscriptMessage};
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
    /// Reply to a message outside the window: its author and quote.
    old_reply: Option<(AuthorIdentity, String)>,
    /// Forwarded text and the preview of its link.
    forward: Option<(String, EmbedData)>,
}

fn line(minutes_ago: i64, author: &AuthorIdentity, text: &str) -> Line {
    Line {
        minutes_ago,
        author: author.clone(),
        text: text.into(),
        reply_to: None,
        old_reply: None,
        forward: None,
    }
}

fn replying(mut line: Line, target: usize) -> Line {
    line.reply_to = Some(target);
    line
}

fn preview(url: &str, description: &str) -> EmbedData {
    EmbedData {
        url: Some(url.into()),
        description: Some(description.into()),
        ..Default::default()
    }
}

/// Quote of a message, as built from a Discord reply payload.
fn quote(line: &Line) -> String {
    match &line.forward {
        Some((text, e)) => format!(
            "[transféré] {text} [aperçu : {}]",
            e.description.clone().unwrap_or_default()
        ),
        None => line.text.clone(),
    }
}

/// The last line is the trigger.
fn context(lines: Vec<Line>) -> ChannelContext {
    let now = chrono::Utc::now().timestamp();
    let last = lines.len() - 1;
    let id = |i: usize| serenity::MessageId::new(10_000 + i as u64);
    let messages = lines
        .iter()
        .enumerate()
        .map(|(i, l)| TranscriptMessage {
            id: id(i),
            timestamp: serenity::Timestamp::from_unix_timestamp(now - l.minutes_ago * 60).unwrap(),
            author: l.author.clone(),
            content: l.text.clone(),
            is_trigger: i == last,
            is_reply_target: lines[last].reply_to == Some(i),
            reply_to: l
                .reply_to
                .map(|t| (id(t), Some(lines[t].author.clone())))
                .or_else(|| {
                    l.old_reply
                        .as_ref()
                        .map(|(a, _)| (serenity::MessageId::new(1), Some(a.clone())))
                }),
            reply_quote: l
                .reply_to
                .map(|t| quote(&lines[t]))
                .or_else(|| l.old_reply.as_ref().map(|(_, q)| q.clone())),
            forwarded: l.forward.iter().map(|(t, _)| t.clone()).collect(),
            attachments: vec![],
            stickers: vec![],
            embeds: l.forward.iter().map(|(_, e)| e.clone()).collect(),
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

    // A "." with the mention, answering a message inside the window.
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
        replying(line(0, &bapt, &format!("{} .", mention())), 0),
    ]);

    // A "." answering a message older than the window: its quote is all the
    // model has to understand what it points at.
    let mut dot = line(30, &bapt, ".");
    dot.old_reply = Some((
        bot.clone(),
        "https://fxtwitter.com/devguy/status/1 [aperçu : Benchmarks are an art form — Nice SWE score! Does it survive a lowercase prompt?]".into(),
    ));
    let dot_old = context(vec![
        line(
            40,
            &julie,
            "le nouveau modèle de code est sorti, il explose tous les benchmarks",
        ),
        line(35, &max, "encore un ?"),
        dot,
        line(
            10,
            &kevin,
            "quelqu'un sait si le serveur Minecraft marche encore ?",
        ),
        replying(
            line(0, &max, &format!("occupe toi de lui {}", mention())),
            2,
        ),
    ]);

    // A forward keeps its content in a snapshot: the message itself is empty.
    let mut forward = line(5, &thomas, "");
    forward.forward = Some((
        "https://vxtwitter.com/artstudio/status/2".into(),
        preview(
            "https://vxtwitter.com/artstudio/status/2",
            "Niji V8 is now live! This is our latest anime model with precision prompting, consistent characters and a dedicated editing model. Have fun!",
        ),
    ));
    let forward_weeb = context(vec![
        line(8, &julie, "vous faites quoi ce week-end ?"),
        forward,
        replying(line(0, &bapt, &format!("{} roast ce weeb", mention())), 1),
    ]);

    // A link without Discord preview: the bot reads the page itself.
    let bench = context(vec![
        line(
            35,
            &kevin,
            "nouvelle config gaming, regardez le benchmark de ma carte, ça va tout défoncer https://www.techpowerup.com/gpu-specs/geforce-gt-1030.c2954",
        ),
        line(33, &julie, "quelqu'un a vu le match hier ?"),
        line(32, &max, "ouais l'arbitrage était horrible"),
        line(20, &thomas, "je commande des pizzas, qui en veut ?"),
        line(19, &julie, "moi !"),
        replying(line(0, &bapt, &format!("{} .", mention())), 0),
    ]);

    let weeb = context(vec![
        line(6, &julie, "vous faites quoi ce soir ?"),
        line(
            5,
            &thomas,
            "je commence celui-là, 170 épisodes ça passe crème https://myanimelist.net/anime/34572",
        ),
        line(4, &max, "courage"),
        replying(line(0, &bapt, &format!("{} roast ce weeb", mention())), 1),
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

    let ask_ctx = context(vec![
        line(5, &max, "ce soir je fais une raclette"),
        line(4, &julie, "en novembre ? t'es sérieux"),
        line(3, &max, "y a pas de saison pour le fromage"),
    ]);

    let only = std::env::var("EVAL_CASE").ok();
    let wanted = |name: &str| {
        only.as_deref()
            .is_none_or(|o| o.split(',').any(|c| c == name))
    };
    let print = |name: &str, run: usize, out: String| println!("\n=== {name} #{run}\n{out}");

    for (name, ctx) in [
        ("reply", &reply),
        ("dot_old", &dot_old),
        ("forward", &forward_weeb),
        ("bench", &bench),
        ("weeb", &weeb),
        ("general", &general),
        ("target", &target),
    ] {
        for run in (1..=2).filter(|_| wanted(name)) {
            let out = crate::agents::roast_channel(&llm, ctx).await;
            print(
                name,
                run,
                out.map(|o| o.response)
                    .unwrap_or_else(|e| format!("ERROR {e}")),
            );
        }
    }
    for run in (1..=2).filter(|_| wanted("truth")) {
        let out = crate::agents::roast_truth(&llm, &truth).await;
        print(
            "truth",
            run,
            out.map(|o| o.response)
                .unwrap_or_else(|e| format!("ERROR {e}")),
        );
    }
    for run in (1..=2).filter(|_| wanted("ask")) {
        let out = crate::agents::ask(
            &llm,
            &max,
            "combien de calories dans une raclette ?",
            &ask_ctx,
        )
        .await;
        print("ask", run, out.unwrap_or_else(|e| format!("ERROR {e}")));
    }
}

/// Replays a real Discord conversation: prints the transcript the model
/// gets and two roasts. Nothing is posted.
#[tokio::test]
#[ignore]
async fn replay() {
    use std::collections::HashMap;

    let config = Config::from_env().expect("env");
    let llm = LlmService::new(&config);
    let path = std::env::var("REPLAY_JSON").expect("REPLAY_JSON");
    let trigger_id: serenity::MessageId = std::env::var("REPLAY_TRIGGER")
        .expect("REPLAY_TRIGGER")
        .parse::<u64>()
        .expect("message id")
        .into();
    let self_id: serenity::UserId = std::env::var("REPLAY_SELF_ID")
        .expect("REPLAY_SELF_ID")
        .parse::<u64>()
        .expect("user id")
        .into();
    let mut raw: Vec<serenity::Message> =
        serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("messages");
    raw.sort_by_key(|m| (m.timestamp.unix_timestamp(), m.id.get()));
    let trigger_pos = raw
        .iter()
        .position(|m| m.id == trigger_id)
        .expect("trigger");
    let trigger = raw[trigger_pos].clone();
    // Same window as fetch_channel_context: the messages before the trigger,
    // the trigger, and the replied message if it is older.
    let mut window: Vec<_> = raw[trigger_pos.saturating_sub(20)..trigger_pos].to_vec();
    if let Some(replied) = trigger.referenced_message.as_deref()
        && !window.iter().any(|m| m.id == replied.id)
    {
        window.push(replied.clone());
    }
    window.sort_by_key(|m| (m.timestamp.unix_timestamp(), m.id.get()));
    window.push(trigger.clone());

    let identities: HashMap<_, _> = window
        .iter()
        .map(|m| {
            (
                m.author.id,
                AuthorIdentity {
                    id: m.author.id,
                    name: m
                        .author
                        .global_name
                        .clone()
                        .unwrap_or_else(|| m.author.name.clone()),
                    is_bot: m.author.bot,
                    is_self: m.author.id == self_id,
                },
            )
        })
        .collect();
    let reply_target = crate::bot::context::reply_id(&trigger);
    let context = ChannelContext {
        guild_name: "serveur".into(),
        channel_name: "salon".into(),
        channel_description: None,
        messages: window
            .iter()
            .map(|m| {
                crate::bot::context::transcript_message(
                    m,
                    &identities,
                    self_id,
                    Some(trigger_id),
                    reply_target,
                )
            })
            .collect(),
    };
    let mut shown = context.clone();
    crate::agents::links::add_link_previews(&mut shown).await;
    println!("{}", crate::bot::context::formatter::format_header(&shown));
    for block in crate::bot::context::formatter::format_messages(&shown) {
        print!("{block}");
    }
    if let Some(hint) = crate::agents::roast::reply_hint(&shown) {
        println!("INDICE : {hint}");
    }
    for run in 1..=2 {
        let out = crate::agents::roast_channel(&llm, &context).await;
        println!(
            "\n=== replay #{run}\n{}",
            out.map(|o| o.response)
                .unwrap_or_else(|e| format!("ERROR {e}"))
        );
    }
}
