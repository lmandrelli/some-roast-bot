use async_trait::async_trait;

use crate::bot::handler::{HandlerContext, MessageHandler};
use crate::error::BotError;

fn normalize_lookalikes(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '\u{043E}' => 'o', // Cyrillic о → o
            '\u{0430}' => 'a', // Cyrillic а → a
            '\u{0456}' => 'i', // Cyrillic і → i
            '\u{0441}' => 'c', // Cyrillic с → c
            '\u{0435}' => 'e', // Cyrillic е → e
            '\u{043D}' => 'H', // Cyrillic н → H
            '\u{0437}' => '3', // Cyrillic з → 3
            '0' => 'o',
            '1' => 'i',
            '3' => 'e',
            _ => c.to_ascii_lowercase(),
        })
        .collect()
}

fn contains_quoi(content: &str) -> bool {
    let normalized = normalize_lookalikes(content);
    let trimmed =
        normalized.trim_end_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace());
    trimmed.ends_with("quoi") && !trimmed.ends_with("pourquoi")
}

/// Handler for "quoi" → "-feur" replies.
pub struct QuoiHandler;

#[async_trait]
impl MessageHandler for QuoiHandler {
    fn name(&self) -> &'static str {
        "quoi"
    }

    fn priority(&self) -> u8 {
        1
    }

    async fn can_handle(&self, ctx: &HandlerContext<'_>) -> bool {
        contains_quoi(&ctx.message.content)
    }

    async fn handle(&self, ctx: &HandlerContext<'_>) -> Result<Option<String>, BotError> {
        ctx.memory
            .increment_quoi_feur_count()
            .map_err(|e| BotError::Db(e))?;
        Ok(Some("-feur".to_string()))
    }
}
