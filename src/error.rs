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
// LLM provider error classification (Discord-friendly responses)
// ---------------------------------------------------------------------------

/// Classified LLM error types with their associated GIFs and messages.
///
/// Covers the OpenRouter error code surface:
/// <https://openrouter.ai/docs/api/reference/errors-and-debugging>
#[derive(Debug, Clone)]
pub enum LlmErrorType {
    OutOfTokens,
    ServerOutOfCapacity,
    BadAuthentication,
    PaymentRequired,
    Moderation,
    EmptyResponse,
    Other,
}

impl LlmErrorType {
    pub fn gif_url(&self) -> &'static str {
        match self {
            LlmErrorType::OutOfTokens => {
                "https://klipy.com/gifs/spending-all-your-financial-aid-money-before-classes-even-start"
            }
            LlmErrorType::ServerOutOfCapacity => "https://klipy.com/gifs/dead-server-loki",
            LlmErrorType::BadAuthentication => "https://klipy.com/gifs/let-me-in-eric-andre-3",
            LlmErrorType::PaymentRequired => "https://klipy.com/gifs/no-money-broke-6",
            LlmErrorType::Moderation => "https://klipy.com/gifs/no-no-no-no-21",
            LlmErrorType::EmptyResponse => "https://klipy.com/gifs/empty-inside-sad",
            LlmErrorType::Other => "https://klipy.com/gifs/eroor",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            LlmErrorType::OutOfTokens => {
                "Oh non ! J'ai mangé tous les tokens ! Vous voulez pas me soutenir sur Tipee ?"
            }
            LlmErrorType::ServerOutOfCapacity => {
                "Les serveurs sont saturés... Même OpenRouter a besoin d'une pause café."
            }
            LlmErrorType::BadAuthentication => {
                "Problème d'authentification... Laisse-moi entrer, j'ai perdu mes clés !"
            }
            LlmErrorType::PaymentRequired => {
                "Mon propriétaire a oublié de créditer le compte OpenRouter... OFFREZ-LUI UN CAFÉ !"
            }
            LlmErrorType::Moderation => {
                "OpenRouter ou un provider a bloqué la requête (modération ou guardrail). Pas moi, c'est eux."
            }
            LlmErrorType::EmptyResponse => {
                "Imagine payer pour une requête API, et que tu n'ais rien à recevoir en retour. C'est ma situation. Fun."
            }
            LlmErrorType::Other => {
                "Oups, quelque chose s'est mal passé ! Mais je suis toujours plus stable que Windows."
            }
        }
    }
}

/// Classify an error into an LLM error type by inspecting the error chain.
///
/// Order of checks matters: explicit HTTP status codes are checked first
/// because `rig-core` (via `reqwest`) surfaces them in error Display strings
/// reliably, then OpenRouter-specific `error_type` metadata strings, then
/// broader message-substring fallbacks. This matches the OpenRouter error
/// reference at <https://openrouter.ai/docs/api/reference/errors-and-debugging>.
pub fn classify_llm_error(error: &dyn StdError) -> LlmErrorType {
    let error_string = format!("{:#}", error);
    let lower = error_string.to_lowercase();

    let mut current: Option<&dyn StdError> = Some(error);
    while let Some(err) = current {
        let msg = err.to_string().to_lowercase();
        let combined = format!("{msg} {lower}");

        // --- Context length / token limit (typically 400) ---
        if msg.contains("context length exceeded")
            || msg.contains("maximum context length")
            || msg.contains("token limit")
            || msg.contains("too many tokens")
            || msg.contains("max_tokens")
            || lower.contains("context_length_exceeded")
        {
            return LlmErrorType::OutOfTokens;
        }

        // --- HTTP status code checks (most reliable) ---
        // 401: invalid credentials
        if contains_http_status(&combined, 401) {
            return LlmErrorType::BadAuthentication;
        }
        // 402: insufficient credits
        if contains_http_status(&combined, 402)
            || lower.contains("payment_required")
            || lower.contains("insufficient credits")
            || lower.contains("out of credits")
        {
            return LlmErrorType::PaymentRequired;
        }
        // 403: forbidden — guardrail or moderation block, NOT auth
        if contains_http_status(&combined, 403)
            || lower.contains("permission_denied")
            || lower.contains("moderation")
            || lower.contains("guardrail")
            || lower.contains("flagged")
            || lower.contains("prompt injection")
            || lower.contains("request blocked")
        {
            return LlmErrorType::Moderation;
        }
        // 408: request timeout (transient)
        // 429: rate limited
        // 502: upstream provider down / invalid response
        // 503: no available provider meeting routing
        if contains_http_status(&combined, 408)
            || contains_http_status(&combined, 429)
            || contains_http_status(&combined, 502)
            || contains_http_status(&combined, 503)
            || lower.contains("rate_limit_exceeded")
            || lower.contains("provider_overloaded")
            || lower.contains("provider_unavailable")
            || lower.contains("rate limit")
            || lower.contains("too many requests")
            || lower.contains("request timed out")
            || lower.contains("request timeout")
            || lower.contains("service unavailable")
            || lower.contains("no available")
            || lower.contains("overloaded")
            || lower.contains("server error")
        {
            return LlmErrorType::ServerOutOfCapacity;
        }
        // 400: bad request (our bug or invalid input)
        if contains_http_status(&combined, 400) {
            return LlmErrorType::Other;
        }

        // --- Generic auth fallbacks (no status code in string) ---
        if lower.contains("invalid_api_key")
            || lower.contains("authentication_error")
            || (msg.contains("unauthorized") && !msg.contains("403"))
            || (msg.contains("invalid api key") && !msg.contains("403"))
        {
            return LlmErrorType::BadAuthentication;
        }

        // --- Empty / no-content responses ---
        if msg.contains("no message or tool call")
            || msg.contains("response contained no message")
            || lower.contains("empty")
        {
            return LlmErrorType::EmptyResponse;
        }

        current = err.source();
    }

    LlmErrorType::Other
}

/// Check whether an HTTP status code appears as a standalone token in `s`.
/// Matches "401", " 401 ", "401:", "(401)", "[401]", "401\n" etc., avoiding
/// false positives like "1401" or "40100".
fn contains_http_status(s: &str, code: u16) -> bool {
    let needle = code.to_string();
    let bytes = s.as_bytes();
    let mut start = 0;
    while let Some(idx) = s[start..].find(&needle) {
        let abs = start + idx;
        let before_ok = abs == 0 || !bytes[abs - 1].is_ascii_digit();
        let end = abs + needle.len();
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_digit();
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    fn make_err(msg: &str) -> io::Error {
        io::Error::new(io::ErrorKind::Other, msg)
    }

    #[test]
    fn status_401_is_auth() {
        let e = make_err("HTTP status client error (401 Unauthorized)");
        assert!(matches!(
            classify_llm_error(&e),
            LlmErrorType::BadAuthentication
        ));
    }

    #[test]
    fn status_402_is_payment() {
        let e = make_err("HTTP status client error (402 Payment Required)");
        assert!(matches!(
            classify_llm_error(&e),
            LlmErrorType::PaymentRequired
        ));
    }

    #[test]
    fn status_403_is_moderation_not_auth() {
        let e = make_err("HTTP status client error (403 Forbidden)");
        assert!(matches!(classify_llm_error(&e), LlmErrorType::Moderation));
    }

    #[test]
    fn status_408_is_capacity() {
        let e = make_err("HTTP status client error (408 Request Timeout)");
        assert!(matches!(
            classify_llm_error(&e),
            LlmErrorType::ServerOutOfCapacity
        ));
    }

    #[test]
    fn status_429_is_capacity() {
        let e = make_err("HTTP status client error (429 Too Many Requests)");
        assert!(matches!(
            classify_llm_error(&e),
            LlmErrorType::ServerOutOfCapacity
        ));
    }

    #[test]
    fn status_502_is_capacity() {
        let e = make_err("HTTP status server error (502 Bad Gateway)");
        assert!(matches!(
            classify_llm_error(&e),
            LlmErrorType::ServerOutOfCapacity
        ));
    }

    #[test]
    fn status_503_is_capacity() {
        let e = make_err("HTTP status server error (503 Service Unavailable)");
        assert!(matches!(
            classify_llm_error(&e),
            LlmErrorType::ServerOutOfCapacity
        ));
    }

    #[test]
    fn openrouter_no_available_message_is_capacity() {
        let e =
            make_err("There is no available model provider that meets your routing requirements");
        assert!(matches!(
            classify_llm_error(&e),
            LlmErrorType::ServerOutOfCapacity
        ));
    }

    #[test]
    fn openrouter_insufficient_credits_is_payment() {
        let e = make_err("Your account or API key has insufficient credits");
        assert!(matches!(
            classify_llm_error(&e),
            LlmErrorType::PaymentRequired
        ));
    }

    #[test]
    fn openrouter_guardrail_block_is_moderation() {
        let e = make_err("Request blocked: prompt injection patterns detected");
        assert!(matches!(classify_llm_error(&e), LlmErrorType::Moderation));
    }

    #[test]
    fn context_length_is_out_of_tokens() {
        let e = make_err("context_length_exceeded: maximum context length is 8192 tokens");
        assert!(matches!(classify_llm_error(&e), LlmErrorType::OutOfTokens));
    }

    #[test]
    fn substring_1401_does_not_match_401() {
        let s = "request id 1401 logged";
        assert!(!contains_http_status(s, 401));
    }

    #[test]
    fn substring_40100_does_not_match_401() {
        let s = "status 40100 returned";
        assert!(!contains_http_status(s, 401));
    }

    #[test]
    fn status_401_with_brackets_matches() {
        let s = "error: [401] unauthorized";
        assert!(contains_http_status(s, 401));
    }
}

/// Build a Discord-formatted error response with the GIF and message.
pub fn discord_error_response(error: &dyn StdError) -> String {
    let classified = classify_llm_error(error);
    format!("{}\n{}", classified.message(), classified.gif_url())
}
