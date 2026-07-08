use super::{FixedLink, Platform};
use regex::Regex;
use std::time::Duration;

static FIX_BASES: &[&str] = &[
    "https://vxinstagram.com",
    "https://www.instagram7.com",
    "https://kkinstagram.com",
];

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
        if has_embed_metadata(client, url).await {
            return Some(url.clone());
        }
    }

    tracing::warn!("No Instagram embed provider passed preflight for {:?}", urls);
    None
}

async fn has_embed_metadata(client: &reqwest::Client, url: &str) -> bool {
    let Ok(resp) = client
        .get(url)
        .header("User-Agent", "some-roast-bot/0.5.0")
        .send()
        .await
    else {
        return false;
    };

    if !resp.status().is_success() {
        return false;
    }

    let Ok(body) = resp.text().await else {
        return false;
    };

    body.contains("og:image")
        || body.contains("og:video")
        || body.contains("twitter:image")
        || body.contains("twitter:player")
}

#[cfg(test)]
mod tests {
    use super::extract_links;

    #[test]
    fn preserves_reel_route_for_all_providers() {
        let links = extract_links("https://www.instagram.com/reel/ABC_123-");

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].candidates[0], "https://vxinstagram.com/reel/ABC_123-");
        assert_eq!(
            links[0].candidates[1],
            "https://www.instagram7.com/reel/ABC_123-"
        );
        assert_eq!(links[0].candidates[2], "https://kkinstagram.com/reel/ABC_123-");
    }

    #[test]
    fn treats_short_instagram_links_as_posts() {
        let links = extract_links("https://instagr.am/p/XYZ987");

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].candidates[0], "https://vxinstagram.com/p/XYZ987");
    }
}
