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
/// Twitter and Bluesky calls are issued concurrently; the rest are synchronous.
pub async fn fix_links(text: &str) -> Vec<FixedLink> {
    let mut results = Vec::new();

    // Platforms that need an async API call (concurrent)
    let (twitter_res, bluesky_res) = tokio::join!(
        twitter::fix(text),
        bluesky::fix(text),
    );
    results.extend(twitter_res);
    results.extend(bluesky_res);

    // Platforms that are pure regex rewrite (sync)
    results.extend(instagram::fix(text));
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
