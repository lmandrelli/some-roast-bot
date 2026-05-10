use super::{FixedLink, Platform};
use regex::Regex;

static FIX_BASE: &str = "https://tnktok.com";

/// Extract TikTok links and rewrite them to tnktok.com.
pub fn fix(text: &str) -> Vec<FixedLink> {
    let re = Regex::new(r#"https?://(?:www\.)?tiktok\.com/@([\w.\-]+)/video/(\d+)"#).unwrap();

    let mut results = Vec::new();

    for caps in re.captures_iter(text) {
        let original = caps.get(0).unwrap().as_str().to_string();
        let user = &caps[1];
        let video_id = &caps[2];

        results.push(FixedLink {
            platform: Platform::TikTok,
            original_url: original,
            fixed_url: format!("{}/@{}/video/{}", FIX_BASE, user, video_id),
            translated: false,
        });
    }

    results
}
