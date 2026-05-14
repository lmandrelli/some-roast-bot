use serde::Deserialize;

/// Generic FxEmbed API response.
/// Used for both Twitter/X (api.fxtwitter.com) and Bluesky (api.fxbsky.app).
#[derive(Debug, Deserialize)]
pub struct FxEmbedResponse {
    pub code: u16,
    #[serde(default)]
    pub message: String,
    /// Twitter/X v1 content (legacy)
    pub tweet: Option<FxContent>,
    /// Bluesky v1 content (legacy)
    pub post: Option<FxContent>,
    /// V2 API content (used by both Twitter/X and Bluesky v2 endpoints)
    pub status: Option<FxContent>,
}

#[derive(Debug, Deserialize)]
pub struct FxContent {
    /// ISO 639-1 language code, e.g. "en", "fr", "es"
    pub lang: Option<String>,
}
