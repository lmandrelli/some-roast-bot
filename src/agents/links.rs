//! Link previews for the messages the model must understand. Discord adds
//! its own preview (embed) later than the gateway event, or not at all, so
//! the trigger and the message it replies to often reach the bot without
//! one: the bot then reads the page title and description itself.

use std::{
    net::{IpAddr, SocketAddr},
    sync::LazyLock,
    time::Duration,
};

use futures_util::{StreamExt as _, future::join_all};
use regex::Regex;

use crate::bot::context::{ChannelContext, EmbedData};

const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
/// Title and Open Graph tags sit in the head of the page.
const PAGE_BYTE_LIMIT: usize = 512 * 1024;
const LINK_LIMIT: usize = 3;
const REDIRECT_LIMIT: usize = 3;
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
        let text = std::iter::once(&message.content)
            .chain(&message.forwarded)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n");
        for url in urls(&text) {
            let previewed = message.embeds.iter().any(|e| {
                e.url.as_deref() == Some(url.as_str())
                    && (e.title.is_some() || e.description.is_some())
            });
            if !previewed && wanted.len() < LINK_LIMIT && !wanted.iter().any(|(_, u)| *u == url) {
                wanted.push((index, url));
            }
        }
    }
    if wanted.is_empty() {
        return;
    }
    let previews = join_all(wanted.into_iter().map(|(index, url)| async move {
        let preview = tokio::time::timeout(FETCH_TIMEOUT, fetch_preview(&url))
            .await
            .unwrap_or(Err("timeout"));
        (index, preview, url)
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

/// Only public hosts, before any DNS lookup: no localhost or `.local` name,
/// no IP literal of a private, loopback, link-local or CGNAT range. The
/// addresses a name resolves to are checked again in `resolve_public`.
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
            if let Some(v4) = ip.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(v4));
            }
            !(ip.is_loopback()
                || ip.is_unspecified()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
                || (ip.segments()[0] & 0xffc0) == 0xfe80)
        }
    }
}

/// The address to connect to: every address the host resolves to must be
/// public (a name pointing at a private address is refused, even if it
/// also has a public one).
fn pick_public(addrs: &[SocketAddr]) -> Result<SocketAddr, &'static str> {
    match addrs.first() {
        None => Err("dns"),
        Some(_) if addrs.iter().any(|a| !is_public_ip(a.ip())) => Err("private-address"),
        Some(addr) => Ok(*addr),
    }
}

async fn resolve_public(url: &url::Url) -> Result<SocketAddr, &'static str> {
    if !is_public(url) {
        return Err("private-host");
    }
    let host = url.host_str().ok_or("url")?;
    let port = url.port_or_known_default().ok_or("url")?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| "dns")?
        .collect();
    pick_public(&addrs)
}

/// Fetch the page and read its preview tags. Each hop (redirects included,
/// at most `REDIRECT_LIMIT`) is resolved and checked first, then the client
/// connects to that checked address only, so a DNS answer cannot change
/// between the check and the connection.
async fn fetch_preview(source: &str) -> Result<EmbedData, &'static str> {
    let mut url = url::Url::parse(source).map_err(|_| "url")?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("scheme");
    }
    for _ in 0..=REDIRECT_LIMIT {
        let addr = resolve_public(&url).await?;
        let mut builder = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(FETCH_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none());
        if let Some(url::Host::Domain(domain)) = url.host() {
            builder = builder.resolve(domain, addr);
        }
        let client = builder.build().map_err(|_| "client")?;
        let response = client
            .get(url.clone())
            .send()
            .await
            .map_err(|_| "request")?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or("redirect")?;
            url = url.join(location).map_err(|_| "redirect")?;
            if !matches!(url.scheme(), "http" | "https") {
                return Err("scheme");
            }
            continue;
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
        let preview = parse_preview(&html, &url);
        if preview.title.is_none() && preview.description.is_none() {
            return Err("no-preview");
        }
        return Ok(EmbedData {
            url: Some(source.to_string()),
            ..preview
        });
    }
    Err("too-many-redirects")
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
        assert!(!is_public_ip("::ffff:127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn a_name_resolving_to_any_private_address_is_refused() {
        let public: SocketAddr = "93.184.215.14:80".parse().unwrap();
        let private: SocketAddr = "127.0.0.1:80".parse().unwrap();
        assert_eq!(pick_public(&[public]), Ok(public));
        assert_eq!(pick_public(&[private]), Err("private-address"));
        assert_eq!(pick_public(&[public, private]), Err("private-address"));
        assert_eq!(pick_public(&[]), Err("dns"));
    }

    /// Private targets must be refused before any request reaches them.
    /// The local server counts its hits.
    #[tokio::test]
    async fn private_targets_are_never_contacted() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = std::sync::Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                counter.fetch_add(1, Ordering::SeqCst);
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let _ = socket
                    .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/html\r\n\r\n<title>internal</title>")
                    .await;
            }
        });
        for url in [
            format!("http://127.0.0.1:{port}/"),
            format!("http://localhost:{port}/"),
            format!("http://[::ffff:127.0.0.1]:{port}/"),
        ] {
            assert!(fetch_preview(&url).await.is_err(), "{url}");
        }
        assert_eq!(hits.load(Ordering::SeqCst), 0);
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
