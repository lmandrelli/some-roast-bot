use super::{FixedLink, Platform, should_translate};
use crate::models::fxtwitter::FxEmbedResponse;
use regex::Regex;

static API_BASE: &str = "https://api.fxbsky.app";
static FIX_BASE: &str = "https://fxbsky.app";

/// Extract Bluesky links and rewrite them with smart translation.
pub async fn fix(text: &str) -> Vec<FixedLink> {
    let re = Regex::new(
        r#"https?://(?:www\.)?bsky\.app/profile/([\w.\-]+)/post/([\w]+)"#
    ).unwrap();

    let mut results = Vec::new();

    for caps in re.captures_iter(text) {
        let original = caps.get(0).unwrap().as_str().to_string();
        let handle = &caps[1];
        let rkey = &caps[2];

        let api_url = format!("{}/profile/{}/post/{}", API_BASE, handle, rkey);
        let (fixed_url, translated) = match fetch_lang(&api_url).await {
            Some(lang) if should_translate(Some(&lang)) => {
                (format!("{}/profile/{}/post/{}/fr", FIX_BASE, handle, rkey), true)
            }
            _ => {
                (format!("{}/profile/{}/post/{}", FIX_BASE, handle, rkey), false)
            }
        };

        results.push(FixedLink {
            platform: Platform::Bluesky,
            original_url: original,
            fixed_url,
            translated,
        });
    }

    results
}

/// Query the FxEmbed API and return the post language code.
async fn fetch_lang(api_url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()?;

    let resp = client.get(api_url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let data: FxEmbedResponse = resp.json().await.ok()?;
    if data.code != 200 {
        return None;
    }

    data.post.and_then(|p| p.lang)
}
