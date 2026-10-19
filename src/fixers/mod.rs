pub mod bluesky;
pub mod instagram;
pub mod reddit;
pub mod tiktok;
pub mod twitter;

/// A fixed social-media link ready to be posted back to Discord.
#[derive(Debug, Clone)]
pub struct FixedLink {
    pub platform: Platform,
    pub original_url: String,
    pub fixed_url: String,
    pub translated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Platform {
    Twitter,
    Bluesky,
    Instagram,
    Reddit,
    TikTok,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Twitter => write!(f, "Twitter"),
            Platform::Bluesky => write!(f, "Bluesky"),
            Platform::Instagram => write!(f, "Instagram"),
            Platform::Reddit => write!(f, "Reddit"),
            Platform::TikTok => write!(f, "TikTok"),
        }
    }
}

/// Detect and rewrite every fixable link in `text`.
///
/// Network-backed fixers are issued concurrently; the rest are synchronous.
pub async fn fix_links(text: &str) -> Vec<FixedLink> {
    let mut results = Vec::new();

    // Platforms that need an async API call (concurrent)
    let (twitter_res, bluesky_res, instagram_res) =
        tokio::join!(twitter::fix(text), bluesky::fix(text), instagram::fix(text),);
    results.extend(twitter_res);
    results.extend(bluesky_res);
    results.extend(instagram_res);

    // Platforms that are pure regex rewrite (sync)
    results.extend(reddit::fix(text));
    results.extend(tiktok::fix(text));

    results
}

/// Return `true` if the language code indicates we *should* append `/fr`.
/// We skip translation for English and French content.
pub fn should_translate(lang: Option<&str>) -> bool {
    match lang {
        Some("en") | Some("fr") | Some("en-US") | Some("en-GB") | Some("fr-FR") => false,
        Some(_) => true,
        None => false, // unknown language → don't guess
    }
}

/// Cheap content check: does `text` contain a transformed (fixed)
/// social-media URL produced by this bot?
///
/// Used by the handler pipeline to recognise replies to the bot's own
/// social-link reposts without making network calls.
pub fn contains_fixed_link(text: &str) -> bool {
    text.contains("fxtwitter.com")
        || text.contains("fxbsky.app")
        || instagram::FIX_BASES
            .iter()
            .chain(LEGACY_INSTAGRAM_BASES)
            .any(|base| text.contains(base))
        || text.contains("rxddit.com")
        || text.contains("tnktok.com")
}

/// Instagram providers the bot used to post, so replies to older reposts are
/// still recognised.
const LEGACY_INSTAGRAM_BASES: &[&str] = &["https://vxinstagram.com", "https://www.instagram7.com"];

#[cfg(test)]
mod tests {
    use super::contains_fixed_link;

    #[test]
    fn recognises_current_and_legacy_instagram_providers() {
        for url in [
            "https://eeinstagram.com/reel/ABC",
            "https://uuinstagram.com/p/ABC",
            "https://fxinstagram.com/p/ABC",
            "https://zzinstagram.com/p/ABC",
            "https://instagramfix.com/p/ABC",
            "https://kkinstagram.com/reel/ABC",
            "https://vxinstagram.com/p/ABC",
            "https://www.instagram7.com/p/ABC",
        ] {
            assert!(contains_fixed_link(url), "{url}");
        }
    }

    #[test]
    fn ignores_original_instagram_links() {
        assert!(!contains_fixed_link("https://www.instagram.com/p/ABC"));
        assert!(!contains_fixed_link("https://instagram.com/reel/ABC"));
    }
}
