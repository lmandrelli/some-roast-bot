use super::{FixedLink, Platform};
use regex::Regex;
use std::time::Duration;

/// Providers tried in order; each must pass the embed preflight to be used.
/// Validated 2026-10-19 with the Discordbot UA on real posts/reels: all return
/// og:video/og:image for real shortcodes and nothing for unknown ones.
pub(crate) static FIX_BASES: &[&str] = &[
    "https://eeinstagram.com",
    "https://uuinstagram.com",
    "https://fxinstagram.com",
    "https://zzinstagram.com",
    "https://instagramfix.com",
    UNCONDITIONAL_FALLBACK_BASE,
];

/// Last-resort provider, posted without preflight when every other provider
/// failed. kkinstagram redirects Discordbot straight to the media file.
const UNCONDITIONAL_FALLBACK_BASE: &str = "https://kkinstagram.com";

/// Discord renders what its crawler sees, so the preflight impersonates it.
const DISCORDBOT_USER_AGENT: &str =
    "Mozilla/5.0 (compatible; Discordbot/2.0; +https://discordapp.com)";

/// Extract Instagram links and rewrite them to the first provider that looks embeddable.
pub async fn fix(text: &str) -> Vec<FixedLink> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .ok();

    let mut results = Vec::new();

    for link in extract_links(text) {
        let Some(fixed_url) = (match &client {
            Some(client) => first_working_url(client, &link.candidates).await,
            None => None,
        }) else {
            continue;
        };

        results.push(FixedLink {
            platform: Platform::Instagram,
            original_url: link.original_url,
            fixed_url,
            translated: false,
        });
    }

    results
}

#[derive(Debug)]
struct InstagramLink {
    original_url: String,
    candidates: Vec<String>,
}

fn extract_links(text: &str) -> Vec<InstagramLink> {
    let re = Regex::new(
        r#"https?://(?:www\.)?(?:instagram\.com/(p|reel|tv)/([\w-]+)|instagr\.am/p/([\w-]+))"#,
    )
    .unwrap();

    let mut results = Vec::new();

    for caps in re.captures_iter(text) {
        let original = caps.get(0).unwrap().as_str().to_string();
        let route = caps.get(1).map(|m| m.as_str()).unwrap_or("p");

        // Capture group 2 = instagram.com shortcode, group 3 = instagr.am shortcode
        let shortcode = caps
            .get(2)
            .or_else(|| caps.get(3))
            .map(|m| m.as_str())
            .unwrap_or("");

        if shortcode.is_empty() {
            continue;
        }

        results.push(InstagramLink {
            original_url: original,
            candidates: FIX_BASES
                .iter()
                .map(|base| format!("{}/{}/{}", base, route, shortcode))
                .collect(),
        });
    }

    results
}

async fn first_working_url(client: &reqwest::Client, urls: &[String]) -> Option<String> {
    for url in urls {
        if is_unconditional_fallback(url) {
            tracing::warn!(
                "No Instagram embed provider passed preflight, falling back to {}",
                url
            );
            return Some(url.clone());
        }
        if has_embed_metadata(client, url).await {
            return Some(url.clone());
        }
    }

    tracing::warn!(
        "No Instagram embed provider passed preflight for {:?}",
        urls
    );
    None
}

fn is_unconditional_fallback(url: &str) -> bool {
    url.strip_prefix(UNCONDITIONAL_FALLBACK_BASE)
        .is_some_and(|rest| rest.starts_with('/'))
}

async fn has_embed_metadata(client: &reqwest::Client, url: &str) -> bool {
    let Ok(resp) = client
        .get(url)
        .header("User-Agent", DISCORDBOT_USER_AGENT)
        .send()
        .await
    else {
        return false;
    };

    if !resp.status().is_success() {
        return false;
    }

    // Some providers redirect the crawler straight to the CDN media file,
    // which Discord embeds directly. Don't download it.
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if is_media_content_type(content_type) {
        return true;
    }

    let Ok(body) = resp.text().await else {
        return false;
    };

    body.contains("og:image")
        || body.contains("og:video")
        || body.contains("twitter:image")
        || body.contains("twitter:player")
}

fn is_media_content_type(content_type: &str) -> bool {
    content_type.starts_with("video/") || content_type.starts_with("image/")
}

#[cfg(test)]
mod tests {
    use super::{
        FIX_BASES, extract_links, first_working_url, is_media_content_type,
        is_unconditional_fallback,
    };

    #[test]
    fn preserves_reel_route_for_all_providers() {
        let links = extract_links("https://www.instagram.com/reel/ABC_123-");

        assert_eq!(links.len(), 1);
        let expected: Vec<String> = FIX_BASES
            .iter()
            .map(|base| format!("{}/reel/ABC_123-", base))
            .collect();
        assert_eq!(links[0].candidates, expected);
        assert_eq!(
            links[0].candidates[0],
            "https://eeinstagram.com/reel/ABC_123-"
        );
    }

    #[test]
    fn treats_short_instagram_links_as_posts() {
        let links = extract_links("https://instagr.am/p/XYZ987");

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].candidates[0], "https://eeinstagram.com/p/XYZ987");
    }

    #[test]
    fn kkinstagram_is_the_last_provider() {
        assert_eq!(FIX_BASES.last(), Some(&"https://kkinstagram.com"));
        assert!(
            FIX_BASES[..FIX_BASES.len() - 1]
                .iter()
                .all(|base| !is_unconditional_fallback(&format!("{}/p/X", base)))
        );
    }

    #[test]
    fn recognises_unconditional_fallback_urls() {
        assert!(is_unconditional_fallback(
            "https://kkinstagram.com/p/XYZ987"
        ));
        assert!(!is_unconditional_fallback(
            "https://kkinstagram.com.evil/p/X"
        ));
        assert!(!is_unconditional_fallback(
            "https://eeinstagram.com/p/XYZ987"
        ));
    }

    #[test]
    fn accepts_direct_media_responses() {
        assert!(is_media_content_type("video/mp4"));
        assert!(is_media_content_type("image/jpeg"));
        assert!(!is_media_content_type("text/html; charset=utf-8"));
        assert!(!is_media_content_type("application/json"));
    }

    #[tokio::test]
    async fn falls_back_to_kkinstagram_without_preflight() {
        // Unroutable providers fail fast; kkinstagram must be returned anyway.
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(200))
            .build()
            .unwrap();
        let urls = vec![
            "http://127.0.0.1:9/p/XYZ987".to_string(),
            "https://kkinstagram.com/p/XYZ987".to_string(),
        ];

        assert_eq!(
            first_working_url(&client, &urls).await.as_deref(),
            Some("https://kkinstagram.com/p/XYZ987")
        );
    }
}
