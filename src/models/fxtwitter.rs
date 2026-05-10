use serde::Deserialize;

/// Generic FxEmbed API response.
/// Used for both Twitter/X (api.fxtwitter.com) and Bluesky (api.fxbsky.app).
#[derive(Debug, Deserialize)]
pub struct FxEmbedResponse {
    pub code: u16,
    pub message: String,
    /// Twitter/X content (when calling api.fxtwitter.com)
    pub tweet: Option<FxContent>,
    /// Bluesky content (when calling api.fxbsky.app)
    pub post: Option<FxContent>,
}

#[derive(Debug, Deserialize)]
pub struct FxContent {
    /// ISO 639-1 language code, e.g. "en", "fr", "es"
    pub lang: Option<String>,
}
