use super::{FixedLink, Platform};
use regex::Regex;

static FIX_BASE: &str = "https://vxinstagram.com";

/// Extract Instagram links and rewrite them to vxinstagram.com.
pub fn fix(text: &str) -> Vec<FixedLink> {
    let re = Regex::new(
        r#"https?://(?:www\.)?(?:instagram\.com/(p|reel|tv)/([\w-]+)|instagr\.am/p/([\w-]+))"#,
    )
    .unwrap();

    let mut results = Vec::new();

    for caps in re.captures_iter(text) {
        let original = caps.get(0).unwrap().as_str().to_string();

        // Capture group 2 = instagram.com shortcode, group 3 = instagr.am shortcode
        let shortcode = caps
            .get(2)
            .or_else(|| caps.get(3))
            .map(|m| m.as_str())
            .unwrap_or("");

        if shortcode.is_empty() {
            continue;
        }

        results.push(FixedLink {
            platform: Platform::Instagram,
            original_url: original,
            fixed_url: format!("{}/p/{}", FIX_BASE, shortcode),
            translated: false,
        });
    }

    results
}
