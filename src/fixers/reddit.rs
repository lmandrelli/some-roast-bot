use super::{FixedLink, Platform};
use regex::Regex;

static FIX_BASE: &str = "https://rxddit.com";

/// Extract Reddit links and rewrite them to rxddit.com.
pub fn fix(text: &str) -> Vec<FixedLink> {
    let re_long = Regex::new(
        r#"https?://(?:www\.)?reddit\.com/r/(\w+)/comments/(\w+)(?:/\w+)?"#
    ).unwrap();

    let re_short = Regex::new(
        r#"https?://(?:www\.)?redd\.it/(\w+)"#
    ).unwrap();

    let mut results = Vec::new();

    for caps in re_long.captures_iter(text) {
        let original = caps.get(0).unwrap().as_str().to_string();
        let sub = &caps[1];
        let post_id = &caps[2];

        results.push(FixedLink {
            platform: Platform::Reddit,
            original_url: original,
            fixed_url: format!("{}/r/{}/comments/{}", FIX_BASE, sub, post_id),
            translated: false,
        });
    }

    for caps in re_short.captures_iter(text) {
        let original = caps.get(0).unwrap().as_str().to_string();
        let post_id = &caps[1];

        results.push(FixedLink {
            platform: Platform::Reddit,
            original_url: original,
            fixed_url: format!("{}/comments/{}", FIX_BASE, post_id),
            translated: false,
        });
    }

    results
}
