use poise::serenity_prelude as serenity;

use crate::bot::Error;

fn normalize_lookalikes(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '\u{043E}' => 'o',  // Cyrillic о → o
            '\u{0430}' => 'a',  // Cyrillic а → a
            '\u{0456}' => 'i',  // Cyrillic і → i
            '\u{0441}' => 'c',  // Cyrillic с → c
            '\u{0435}' => 'e',  // Cyrillic е → e
            '\u{043D}' => 'H',  // Cyrillic н → H (for "н ой" → "noi" lookalike)
            '\u{0437}' => '3',  // Cyrillic з → 3
            '0' => 'o',  // zero → o
            '1' => 'i',  // one → i (partial leetspeak)
            '3' => 'e',  // three → e
            _ => c.to_ascii_lowercase(),
        })
        .collect()
}

pub fn contains_quoi(content: &str) -> bool {
    let normalized = normalize_lookalikes(content);
    let trimmed = normalized.trim_end_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace());
    trimmed.ends_with("quoi") && !trimmed.ends_with("pourquoi")
}

pub async fn handle_quoi(
    _ctx: &serenity::Context,
    msg: &serenity::Message,
) -> Result<String, Error> {
    crate::memory::record_roast(&msg.author.id.to_string(), None, "quoi");
    Ok("-feur".to_string())
}
