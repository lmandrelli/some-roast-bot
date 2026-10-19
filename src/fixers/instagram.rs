use super::{FailedLink, FixedLink, Platform};
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

/// Cheap check (no network): does `text` contain an Instagram link we know how to fix?
pub fn detect(text: &str) -> bool {
    !extract_links(text).is_empty()
}

/// Extract Instagram links and rewrite them to the first provider that looks embeddable.
///
/// Links for which no provider passes the preflight are returned as failures
/// so the caller can tell the user instead of silently dropping them.
pub async fn fix(text: &str) -> (Vec<FixedLink>, Vec<FailedLink>) {
    let links = extract_links(text);
    if links.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            tracing::error!("Failed to build Instagram HTTP client: {}", e);
            let failed = links
                .into_iter()
                .map(|link| FailedLink {
                    platform: Platform::Instagram,
                    original_url: link.original_url,
                    reason: format!("HTTP client error: {e}"),
                })
                .collect();
            return (Vec::new(), failed);
        }
    };

    let mut fixed = Vec::new();
    let mut failed = Vec::new();

    for link in links {
        match first_working_url(&client, &link.candidates).await {
            Ok(fixed_url) => fixed.push(FixedLink {
                platform: Platform::Instagram,
                original_url: link.original_url,
                fixed_url,
                translated: false,
            }),
            Err(reason) => failed.push(FailedLink {
                platform: Platform::Instagram,
                original_url: link.original_url,
                reason,
            }),
        }
    }

    (fixed, failed)
}

#[derive(Debug)]
struct InstagramLink {
    original_url: String,
    candidates: Vec<String>,
}

fn extract_links(text: &str) -> Vec<InstagramLink> {
    // Instagram now also emits `instagram.com/<username>/reel/<code>` and `/reels/<code>`.
    let re = Regex::new(
        r#"https?://(?:www\.)?(?:instagram\.com/(?:[\w.]+/)?(p|reels?|tv)/([\w-]+)|instagr\.am/p/([\w-]+))"#,
    )
    .unwrap();

    let mut results = Vec::new();

    for caps in re.captures_iter(text) {
        let original = caps.get(0).unwrap().as_str().to_string();
        let route = match caps.get(1).map(|m| m.as_str()) {
            Some("reels") => "reel",
            Some(route) => route,
            None => "p",
        };

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

async fn first_working_url(client: &reqwest::Client, urls: &[String]) -> Result<String, String> {
    let mut reasons = Vec::new();

    for url in urls {
        if is_unconditional_fallback(url) {
            tracing::warn!(
                "No Instagram embed provider passed preflight, falling back to {}",
                url
            );
            return Ok(url.clone());
        }
        match has_embed_metadata(client, url).await {
            Ok(()) => return Ok(url.clone()),
            Err(reason) => {
                tracing::warn!("Instagram provider {} failed preflight: {}", url, reason);
                reasons.push(format!("{url}: {reason}"));
            }
        }
    }

    tracing::warn!(
        "No Instagram embed provider passed preflight for {:?}",
        urls
    );
    Err(reasons.join("; "))
}

fn is_unconditional_fallback(url: &str) -> bool {
    url.strip_prefix(UNCONDITIONAL_FALLBACK_BASE)
        .is_some_and(|rest| rest.starts_with('/'))
}

async fn has_embed_metadata(client: &reqwest::Client, url: &str) -> Result<(), String> {
    let resp = client
        .get(url)
        .header("User-Agent", DISCORDBOT_USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    let status = resp.status();
    let final_url = resp.url().to_string();
    if !status.is_success() {
        return Err(format!("HTTP {status} (final URL {final_url})"));
    }

    // Some providers redirect the crawler straight to the CDN media file,
    // which Discord embeds directly. Don't download it.
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if is_media_content_type(content_type) {
        return Ok(());
    }

    let body = resp
        .text()
        .await
        .map_err(|e| format!("body read failed: {e}"))?;

    if body.contains("og:image")
        || body.contains("og:video")
        || body.contains("twitter:image")
        || body.contains("twitter:player")
    {
        Ok(())
    } else {
        Err(format!("no embed metadata (final URL {final_url})"))
    }
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
    fn handles_username_and_reels_paths() {
        let links = extract_links(
            "https://www.instagram.com/some.user/reel/ABC123/?igsh=xyz https://instagram.com/reels/DEF456",
        );

        assert_eq!(links.len(), 2);
        assert_eq!(
            links[0].candidates[0],
            "https://eeinstagram.com/reel/ABC123"
        );
        assert_eq!(
            links[1].candidates[0],
            "https://eeinstagram.com/reel/DEF456"
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
            first_working_url(&client, &urls).await.ok().as_deref(),
            Some("https://kkinstagram.com/p/XYZ987")
        );
    }
}
