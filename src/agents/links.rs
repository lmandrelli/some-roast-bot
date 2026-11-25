//! Link previews for the messages the model must understand. Discord adds
//! its own preview (embed) later than the gateway event, or not at all, so
//! the trigger and the message it replies to often reach the bot without
//! one: the bot then reads the page title and description itself.

use std::{net::IpAddr, sync::LazyLock, time::Duration};

use futures_util::{StreamExt as _, future::join_all};
use regex::Regex;

use crate::bot::context::{ChannelContext, EmbedData};

const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
/// Title and Open Graph tags sit in the head of the page.
const PAGE_BYTE_LIMIT: usize = 512 * 1024;
const LINK_LIMIT: usize = 3;
/// Same user agent as the Instagram fixer: sites serve their preview tags to it.
const USER_AGENT: &str = "Mozilla/5.0 (compatible; Discordbot/2.0; +https://discordapp.com)";

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"https?://[^\s<>()\[\]"'`]+"#).unwrap());
static TITLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap());
static META_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<meta\s[^>]*>").unwrap());
static ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)\b(property|name|content)\s*=\s*(?:"([^"]*)"|'([^']*)')"#).unwrap()
});

/// Add a preview to the trigger and the message it replies to, for each
/// link Discord did not preview (at most `LINK_LIMIT`). Failures are logged
/// and skipped.
pub async fn add_link_previews(context: &mut ChannelContext) {
    let mut wanted = Vec::new();
    for (index, message) in context.messages.iter().enumerate() {
        if !(message.is_trigger || message.is_reply_target) {
            continue;
        }
        for url in urls(&message.content) {
            let previewed = message
                .embeds
                .iter()
                .any(|e| e.url.as_deref() == Some(url.as_str()) && e.title.is_some());
            if !previewed && wanted.len() < LINK_LIMIT && !wanted.iter().any(|(_, u)| *u == url) {
                wanted.push((index, url));
            }
        }
    }
    if wanted.is_empty() {
        return;
    }
    let Ok(client) = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(FETCH_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
    else {
        return;
    };
    let previews = join_all(wanted.into_iter().map(|(index, url)| {
        let client = &client;
        async move { (index, fetch_preview(client, &url).await, url) }
    }))
    .await;
    for (index, preview, url) in previews {
        match preview {
            Ok(preview) => context.messages[index].embeds.push(preview),
            Err(stage) => tracing::warn!(failure_stage = stage, "link preview skipped for {url}"),
        }
    }
}

fn urls(text: &str) -> Vec<String> {
    URL_RE
        .find_iter(text)
        .map(|m| {
            m.as_str()
                .trim_end_matches(['.', ',', ';', ':', '!', '?', '>', '*', '_', '~', '|'])
                .to_string()
        })
        .collect()
}

/// Only public hosts: no IP literal of a private, loopback or link-local
/// range, no localhost.
fn is_public(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(domain)) => {
            let domain = domain.trim_end_matches('.').to_ascii_lowercase();
            domain != "localhost" && !domain.ends_with(".localhost") && !domain.ends_with(".local")
        }
        Some(url::Host::Ipv4(ip)) => is_public_ip(IpAddr::V4(ip)),
        Some(url::Host::Ipv6(ip)) => is_public_ip(IpAddr::V6(ip)),
        None => false,
    }
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                // Carrier-grade NAT, used by Tailscale.
                || (ip.octets()[0] == 100 && (ip.octets()[1] & 0xc0) == 64))
        }
        IpAddr::V6(ip) => {
            !(ip.is_loopback()
                || ip.is_unspecified()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
                || (ip.segments()[0] & 0xffc0) == 0xfe80)
        }
    }
}

async fn fetch_preview(client: &reqwest::Client, source: &str) -> Result<EmbedData, &'static str> {
    let url = url::Url::parse(source).map_err(|_| "url")?;
    if !is_public(&url) {
        return Err("private-host");
    }
    let response = client.get(url).send().await.map_err(|_| "request")?;
    if !is_public(response.url()) {
        return Err("private-redirect");
    }
    let response = response.error_for_status().map_err(|_| "http-status")?;
    let is_html = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("html"));
    if !is_html {
        return Err("not-html");
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "body")?;
        bytes.extend_from_slice(&chunk);
        if bytes.len() >= PAGE_BYTE_LIMIT {
            break;
        }
    }
    let html = String::from_utf8_lossy(&bytes);
    let preview = parse_preview(&html, &url::Url::parse(source).map_err(|_| "url")?);
    if preview.title.is_none() && preview.description.is_none() {
        return Err("no-preview");
    }
    Ok(EmbedData {
        url: Some(source.to_string()),
        ..preview
    })
}

fn parse_preview(html: &str, url: &url::Url) -> EmbedData {
    let mut meta = std::collections::HashMap::new();
    for tag in META_RE.find_iter(html) {
        let mut key = None;
        let mut content = None;
        for attr in ATTR_RE.captures_iter(tag.as_str()) {
            let value = attr.get(2).or_else(|| attr.get(3)).map(|v| v.as_str());
            match attr[1].to_ascii_lowercase().as_str() {
                "property" | "name" => key = value.map(str::to_ascii_lowercase),
                _ => content = value,
            }
        }
        if let (Some(key), Some(content)) = (key, content) {
            meta.entry(key).or_insert_with(|| decode_entities(content));
        }
    }
    let pick = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| meta.get(*k).cloned())
            .filter(|v| !v.trim().is_empty())
    };
    let title = pick(&["og:title", "twitter:title"]).or_else(|| {
        TITLE_RE
            .captures(html)
            .map(|c| decode_entities(c[1].trim()))
            .filter(|t| !t.is_empty())
    });
    EmbedData {
        provider: pick(&["og:site_name"]).or_else(|| url.host_str().map(str::to_string)),
        title,
        description: pick(&["og:description", "twitter:description", "description"]),
        ..Default::default()
    }
}

fn decode_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_urls_without_trailing_punctuation() {
        assert_eq!(
            urls("regarde https://myanimelist.net/anime/34572. et <https://a.b/c?d=1>!"),
            vec!["https://myanimelist.net/anime/34572", "https://a.b/c?d=1"]
        );
    }

    #[test]
    fn refuses_private_hosts() {
        for url in [
            "http://localhost/x",
            "http://127.0.0.1/",
            "http://10.0.0.1/",
            "http://192.168.1.1/",
            "http://100.100.1.1/",
            "http://[::1]/",
            "http://[fd00::1]/",
            "http://nas.local/",
        ] {
            assert!(!is_public(&url::Url::parse(url).unwrap()), "{url}");
        }
        assert!(is_public(
            &url::Url::parse("https://myanimelist.net/").unwrap()
        ));
        assert!(is_public(&url::Url::parse("https://1.1.1.1/").unwrap()));
    }

    #[test]
    fn parses_open_graph_then_title() {
        let url = url::Url::parse("https://myanimelist.net/anime/34572").unwrap();
        let html = r#"<html><head><title>Ignored</title>
            <meta property="og:title" content="Black Clover - MyAnimeList.net">
            <meta content='Asta &amp; Yuno, 170 episodes' property='og:description'>
            <meta property="og:site_name" content="MyAnimeList.net"></head>"#;
        let preview = parse_preview(html, &url);
        assert_eq!(
            preview.title.as_deref(),
            Some("Black Clover - MyAnimeList.net")
        );
        assert_eq!(
            preview.description.as_deref(),
            Some("Asta & Yuno, 170 episodes")
        );
        assert_eq!(preview.provider.as_deref(), Some("MyAnimeList.net"));

        let bare = parse_preview("<title>\n  Page  &amp; co </title>", &url);
        assert_eq!(bare.title.as_deref(), Some("Page & co"));
        assert_eq!(bare.provider.as_deref(), Some("myanimelist.net"));
        assert_eq!(bare.description, None);
    }
}
