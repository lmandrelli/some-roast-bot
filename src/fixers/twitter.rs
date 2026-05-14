use super::{FixedLink, Platform, should_translate};
use crate::models::fxtwitter::FxEmbedResponse;
use regex::Regex;

static API_BASE: &str = "https://api.fxtwitter.com";
static FIX_BASE: &str = "https://fxtwitter.com";

/// Extract Twitter/X links and rewrite them with smart translation.
pub async fn fix(text: &str) -> Vec<FixedLink> {
    let re =
        Regex::new(r#"https?://(?:www\.)?(?:twitter\.com|x\.com)/([\w_]+)/status/(\d+)"#).unwrap();

    let mut results = Vec::new();

    for caps in re.captures_iter(text) {
        let original = caps.get(0).unwrap().as_str().to_string();
        let user = &caps[1];
        let tweet_id = &caps[2];

        let api_url = format!("{}/2/status/{}", API_BASE, tweet_id);
        let (fixed_url, translated) = match fetch_lang(&api_url).await {
            Some(lang) if should_translate(Some(&lang)) => (
                format!("{}/{}/status/{}/fr", FIX_BASE, user, tweet_id),
                true,
            ),
            _ => (format!("{}/{}/status/{}", FIX_BASE, user, tweet_id), false),
        };

        results.push(FixedLink {
            platform: Platform::Twitter,
            original_url: original,
            fixed_url,
            translated,
        });
    }

    results
}

/// Query the FxEmbed API and return the tweet language code.
/// Returns `None` on any error so we gracefully fall back to the base URL.
async fn fetch_lang(api_url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()?;

    let resp = client
        .get(api_url)
        .header("User-Agent", "some-roast-bot/0.3.0")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let data: FxEmbedResponse = resp.json().await.ok()?;
    if data.code != 200 {
        return None;
    }

    data.status
        .and_then(|s| s.lang)
        .or_else(|| data.tweet.and_then(|t| t.lang))
}
