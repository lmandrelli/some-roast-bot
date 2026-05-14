use std::error::Error as StdError;

#[derive(Debug, thiserror::Error)]
pub enum BotError {
    #[error("Discord API error: {0}")]
    Discord(#[from] serenity::Error),

    #[error("Database error: {0}")]
    Db(#[from] DbError),

    #[error("LLM service error: {0}")]
    Llm(#[from] LlmError),

    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("No handler matched for this message")]
    NoHandler,

    #[error("Unknown error: {0}")]
    Other(String),
}

#[derive(Debug, thiserror::Error)]
#[error("Database operation failed: {0}")]
pub struct DbError(pub rusqlite::Error);

impl From<rusqlite::Error> for DbError {
    fn from(e: rusqlite::Error) -> Self {
        DbError(e)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("MCP connection failed: {0}")]
    McpConnection(String),

    #[error("MCP tool listing failed: {0}")]
    McpToolListing(String),

    #[error("Completion failed: {0}")]
    Completion(String),

    #[error("Empty response from model")]
    EmptyResponse,

    #[error("Parse failed: {0}")]
    Parse(String),
}

// ---------------------------------------------------------------------------
// Chutes-specific error classification (kept for Discord-friendly responses)
// ---------------------------------------------------------------------------

/// Classified Chutes error types with their associated GIFs and messages.
#[derive(Debug, Clone)]
pub enum ChutesErrorType {
    OutOfTokens,
    ServerOutOfCapacity,
    BadAuthentication,
    EmptyResponse,
    Other,
}

impl ChutesErrorType {
    pub fn gif_url(&self) -> &'static str {
        match self {
            ChutesErrorType::OutOfTokens => {
                "https://tenor.com/view/i-declare-bakruptcy-bankrupt-yelling-announce-declare-gif-15663557"
            }
            ChutesErrorType::ServerOutOfCapacity => {
                "https://tenor.com/view/server-down-gif-8526873401543225239"
            }
            ChutesErrorType::BadAuthentication => {
                "https://tenor.com/view/let-me-in-eric-andre-wanna-come-in-gif-13730108"
            }
            ChutesErrorType::EmptyResponse => {
                "https://tenor.com/view/empty-inside-empty-sad-depression-meme-gif-12191495715687738667"
            }
            ChutesErrorType::Other => {
                "https://tenor.com/view/windows-crash-dialog-error-message-popups-many-endless-flood-bsod-microsoft-windows-error-dialog-endless-crashing-blue-screen-of-death-gif-1753725196792798674"
            }
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            ChutesErrorType::OutOfTokens => {
                "Oh non ! J'ai mangé tous les tokens ! Vous voulez pas me soutenir sur Tipee ?"
            }
            ChutesErrorType::ServerOutOfCapacity => {
                "Les serveurs sont saturés... Même Chutes a besoin d'une pause café."
            }
            ChutesErrorType::BadAuthentication => {
                "Problème d'authentification... Laisse-moi entrer, j'ai perdu mes clés !"
            }
            ChutesErrorType::EmptyResponse => {
                "Imagine payer pour une requête API, et que tu n'ais rien à recevoir en retour. C'est ma situation. Fun."
            }
            ChutesErrorType::Other => {
                "Oups, quelque chose s'est mal passé ! Mais je suis toujours plus stable que Windows."
            }
        }
    }
}

/// Classify an error into a Chutes error type by inspecting the error chain.
pub fn classify_chutes_error(error: &dyn StdError) -> ChutesErrorType {
    let error_string = format!("{:#}", error);
    let lower = error_string.to_lowercase();

    let mut current: Option<&dyn StdError> = Some(error);
    while let Some(err) = current {
        let msg = err.to_string().to_lowercase();

        if msg.contains("context length exceeded")
            || msg.contains("maximum context length")
            || msg.contains("token limit")
            || msg.contains("too many tokens")
            || msg.contains("max_tokens")
            || lower.contains("context_length_exceeded")
        {
            return ChutesErrorType::OutOfTokens;
        }

        if msg.contains("503")
            || msg.contains("service unavailable")
            || msg.contains("overloaded")
            || msg.contains("rate limit")
            || msg.contains("too many requests")
            || msg.contains("server error")
            || msg.contains("model overloaded")
            || lower.contains("model_overloaded")
        {
            return ChutesErrorType::ServerOutOfCapacity;
        }

        if msg.contains("401")
            || msg.contains("403")
            || msg.contains("unauthorized")
            || msg.contains("invalid api key")
            || msg.contains("authentication")
            || msg.contains("api key")
            || lower.contains("invalid_api_key")
            || lower.contains("authentication_error")
        {
            return ChutesErrorType::BadAuthentication;
        }

        if msg.contains("no message or tool call")
            || msg.contains("response contained no message")
            || lower.contains("empty")
        {
            return ChutesErrorType::EmptyResponse;
        }

        current = err.source();
    }

    ChutesErrorType::Other
}

/// Build a Discord-formatted error response with the GIF and message.
pub fn discord_error_response(error: &dyn StdError) -> String {
    let classified = classify_chutes_error(error);
    format!("{}\n{}", classified.message(), classified.gif_url())
}
