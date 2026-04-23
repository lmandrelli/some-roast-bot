//! Chutes API error classification and GIF responses.

/// Classified Chutes error types with their associated GIFs and messages.
#[derive(Debug, Clone)]
pub enum ChutesErrorType {
    OutOfTokens,
    ServerOutOfCapacity,
    BadAuthentication,
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
            ChutesErrorType::Other => {
                "https://tenor.com/view/windows-crash-dialog-error-message-popups-many-endless-flood-bsod-microsoft-windows-error-dialog-endless-crashing-blue-screen-of-death-gif-1753725196792798674"
            }
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            ChutesErrorType::OutOfTokens => "Oh non, j'ai plus de tokens ! Même mon cerveau artificiel fait faillite...",
            ChutesErrorType::ServerOutOfCapacity => "Les serveurs sont saturés... Même Chutes a besoin d'une pause café.",
            ChutesErrorType::BadAuthentication => "Problème d'authentification... Laisse-moi entrer, j'ai perdu mes clés !",
            ChutesErrorType::Other => "Oups, quelque chose s'est mal passé ! Même Windows fait moins d'erreurs que ça...",
        }
    }
}

/// Classify an error into a Chutes error type by inspecting the error chain.
pub fn classify_chutes_error(error: &dyn std::error::Error) -> ChutesErrorType {
    let error_string = format!("{:#}", error);
    let lower = error_string.to_lowercase();

    // Check the full error chain for patterns
    let mut current: Option<&dyn std::error::Error> = Some(error);
    while let Some(err) = current {
        let msg = err.to_string().to_lowercase();

        // Out of tokens / context length
        if msg.contains("context length exceeded")
            || msg.contains("maximum context length")
            || msg.contains("token limit")
            || msg.contains("too many tokens")
            || msg.contains("max_tokens")
            || lower.contains("context_length_exceeded")
        {
            return ChutesErrorType::OutOfTokens;
        }

        // Server out of capacity / overloaded
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

        // Bad authentication
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

        current = err.source();
    }

    // Default fallback for any other LLM or generic error
    ChutesErrorType::Other
}

/// Build a Discord-formatted error response with the GIF and message.
pub fn discord_error_response(error: &dyn std::error::Error) -> String {
    let classified = classify_chutes_error(error);
    format!("{}\n{}", classified.message(), classified.gif_url())
}
