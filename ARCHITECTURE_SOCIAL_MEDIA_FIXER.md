# 🏗️ Architecture Proposal — Social Media Embed Fixer
> **Branch:** `feat/social-media-embed-fixer`  
> **Scope:** Auto-detect Twitter/X, Instagram, Bluesky, Reddit, TikTok links → rewrite to embeddable versions with smart translation.  
> **Target version:** `0.2.0`

---

## 1. Goal

When a user posts a social media link, the bot must:
1. **Detect** the link before Discord renders a broken/paywalled embed.
2. **Suppress** the original embed on the user's message.
3. **Rewrite** the URL to an embed-friendly version.
4. **Translate** Twitter/X and Bluesky posts to French (if not already EN/FR).
5. **Reply** with: `Username posted: {fixed_url} ...`
6. **Track edits** — if the user edits their message, update the bot's reply.
7. Provide a **fallback `/fix <url>`** slash command.

---

## 2. Where It Fits in the Existing Pipeline

Current `event_handler` priority in `src/bot/handlers/mod.rs`:
```
0a. "quoi"         → instant reply
0b. Microsoft      → AI roast
0c. truth          → AI judge
--- mention required ---
1.  reply          → AI roast
2.  user mention   → AI roast
3.  channel        → AI roast
```

**New priority — `0d. social media link` (runs FIRST or independently):**
```
0d. social media link → suppress + reply with fixed URL  (NO AI, fast)
```

**Why first?** It is a lightweight, deterministic handler. It does not need the bot to be mentioned. It should not block on AI generation. If it fires, it returns early (before any AI roast logic).

---

## 3. File Layout

```
src/
├── main.rs
├── bot/
│   ├── mod.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── ask.rs
│   │   ├── research.rs
│   │   ├── stats.rs
│   │   └── fix.rs          ← NEW: /fix <url> manual command
│   └── handlers/
│       ├── mod.rs            ← MODIFIED: add link handler call
│       ├── channel.rs
│       ├── microsoft.rs
│       ├── quoi.rs
│       ├── reply.rs
│       ├── truth.rs
│       ├── user.rs
│       └── social_link.rs   ← NEW: main link detection + orchestration
├── fixers/                  ← NEW module
│   ├── mod.rs               ← Public API: `fix_links_in_text(text) -> Vec<FixedLink>`
│   ├── twitter.rs           ← FxEmbed API + lang-gated translation
│   ├── bluesky.rs           ← FxEmbed API + lang-gated translation
│   ├── instagram.rs         ← vxinstagram.com rewrite
│   ├── reddit.rs            ← rxddit.com / fxreddit rewrite
│   └── tiktok.rs            ← tnktok.com rewrite (best effort)
├── models/                  ← NEW module
│   └── fxtwitter.rs         ← serde structs for FxEmbed API response
└── memory.rs                ← MODIFIED: add link-fix stats
```

---

## 4. New Dependencies (`Cargo.toml`)

```toml
[dependencies]
# existing ...
reqwest = { version = "0.12", features = ["json"] }  # HTTP calls to FxEmbed API
url = "2.5"                                           # URL parsing
```

---

## 5. Core Data Types

### `fixers/mod.rs`
```rust
#[derive(Debug, Clone)]
pub struct FixedLink {
    pub platform: Platform,
    pub original_url: String,
    pub fixed_url: String,
    pub translated: bool,      // true if we appended /fr
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Platform {
    Twitter,
    Bluesky,
    Instagram,
    Reddit,
    TikTok,
}

/// Extract all fixable links from raw message text.
pub fn extract_links(text: &str) -> Vec<FixedLink> { ... }
```

### `models/fxtwitter.rs`
```rust
#[derive(Debug, Deserialize)]
pub struct FxTweet {
    pub code: u16,
    pub message: String,
    pub tweet: Option<Tweet>,
}

#[derive(Debug, Deserialize)]
pub struct Tweet {
    pub url: String,
    pub text: String,
    pub author: Author,
    pub lang: Option<String>,   // "en", "fr", "es", ...
    // ... other fields we ignore
}

#[derive(Debug, Deserialize)]
pub struct Author {
    pub name: String,
    pub screen_name: String,
}
```

---

## 6. Per-Platform Logic

### 6.1 Twitter/X (`fixers/twitter.rs`)

| Step | Action |
|------|--------|
| 1 | Regex extract `twitter.com/\w+/status/(\d+)` or `x.com/\w+/status/(\d+)` |
| 2 | Build API URL: `https://api.fxtwitter.com/{user}/status/{id}` |
| 3 | `GET` with `reqwest` (timeout 3s) |
| 4 | Parse JSON → read `tweet.lang` |
| 5 | **IF** `lang` is `Some("en" \| "fr")` → base URL: `fxtwitter.com/...` |
| 6 | **ELSE** → translated URL: `fxtwitter.com/.../fr` |
| 7 | Return `FixedLink { platform: Twitter, translated: ..., ... }` |

**Error handling:** If API fails (timeout, rate-limit, 500), fallback to **base URL without translation**.

**API response example:**
```json
{
  "code": 200,
  "message": "OK",
  "tweet": {
    "url": "https://twitter.com/elonmusk/status/123",
    "text": "...",
    "author": { "name": "Elon Musk", "screen_name": "elonmusk" },
    "lang": "en"
  }
}
```

### 6.2 Bluesky (`fixers/bluesky.rs`)

| Step | Action |
|------|--------|
| 1 | Regex extract `bsky.app/profile/([\w.-]+)/post/([\w]+)` |
| 2 | Build API URL: `https://api.fxbsky.app/profile/{handle}/post/{rkey}` |
| 3 | Same lang-gated logic as Twitter |
| 4 | Translated URL suffix: `/fr` |

**Note:** FxEmbed hosts Bluesky under `fxbsky.app` (or `fxbsky.com`). We'll verify the exact API base during implementation.

### 6.3 Instagram (`fixers/instagram.rs`)

| Step | Action |
|------|--------|
| 1 | Regex extract `instagram.com/(p\|reel\|tv)/([\w-]+)` or `instagr.am/p/([\w-]+)` |
| 2 | Rewrite to: `https://vxinstagram.com/p/{shortcode}` |
| 3 | No API call. No translation. |

### 6.4 Reddit (`fixers/reddit.rs`)

| Step | Action |
|------|--------|
| 1 | Regex extract `reddit.com/r/\w+/comments/(\w+)` or `redd.it/(\w+)` |
| 2 | Rewrite to: `https://rxddit.com/r/{sub}/comments/{id}` or `https://rxddit.com/comments/{id}` |
| 3 | No API call. No translation. |

**Alternative:** `fxreddit.com` if `rxddit.com` breaks.

### 6.5 TikTok (`fixers/tiktok.rs`)

| Step | Action |
|------|--------|
| 1 | Regex extract `tiktok.com/@([\w.-]+)/video/(\d+)` or `vm.tiktok.com/(\w+)` |
| 2 | Rewrite to: `https://tnktok.com/@{user}/video/{id}` |
| 3 | No API call. Best effort. |

---

## 7. Handler Integration (`bot/handlers/social_link.rs`)

```rust
pub async fn handle_social_links(
    ctx: &serenity::Context,
    msg: &serenity::Message,
) -> Result<bool, Error> {
    let fixed = fixers::extract_links(&msg.content);
    if fixed.is_empty() {
        return Ok(false); // no links found, let other handlers run
    }

    // Suppress original embeds on the user's message
    msg.suppress_embeds(&ctx.http).await?;

    // Build reply text
    let urls = fixed.iter()
        .map(|l| l.fixed_url.clone())
        .collect::<Vec<_>>()
        .join(" ");
    let reply_text = format!("{} posted: {}", msg.author.name, urls);

    // Send reply
    msg.reply(&ctx.http, reply_text).await?;

    // Persist mapping for edit tracking (msg.id → bot_reply.id)
    // See §9

    Ok(true) // handled, skip other handlers
}
```

**Integration point in `bot/handlers/mod.rs`:**
```rust
// Priority 0d: Social media links
let has_social_links = social_link::handle_social_links(ctx, new_message).await?;
if has_social_links {
    return Ok(()); // stop here, do not AI-roast
}
```

---

## 8. Embed Suppression Strategy

Discord automatically unfurls links into embeds. We must suppress them **before** Discord renders them (or as soon as possible).

- **`Message::suppress_embeds(&http)`** — removes existing embeds.
- **Timing:** We call this immediately after detecting links, before sending our reply.
- **Race condition risk:** If Discord unfurls before our call, users may see a flash of the broken embed. This is acceptable — the embed will disappear quickly.
- **Permissions:** The bot needs `Manage Messages` permission in the channel to suppress embeds on other users' messages.

---

## 9. Edit Tracking

### Problem
User edits their message (e.g., fixes a typo in the URL). The bot's reply is now stale.

### Solution
Listen for `FullEvent::MessageUpdate`.

```rust
// In event_handler match
FullEvent::MessageUpdate { event, .. } => {
    if let Some(new_content) = &event.content {
        // Lookup original bot reply by original msg.id
        if let Some(bot_reply_id) = memory::get_link_fix_reply(event.id) {
            let fixed = fixers::extract_links(new_content);
            if fixed.is_empty() {
                // User removed all links — delete bot reply?
                // Or edit to "(links removed)"
            } else {
                let urls = fixed.iter().map(|l| l.fixed_url.clone()).collect::<Vec<_>>().join(" ");
                let new_text = format!(
                    "{} posted: {} (modified at: {})",
                    event.author.as_ref().map(|a| a.name.clone()).unwrap_or_default(),
                    urls,
                    chrono::Utc::now().format("%H:%M:%S")
                );
                // Edit the bot's reply
                bot_reply_id.edit(&ctx.http, serenity::EditMessage::new().content(new_text)).await?;
            }
        }
    }
}
```

### Storage
Add to `memory.rs` (SQLite):
```sql
CREATE TABLE IF NOT EXISTS link_fix_replies (
    original_message_id TEXT PRIMARY KEY,
    bot_reply_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

---

## 10. Slash Command `/fix`

```rust
#[poise::command(slash_command, prefix_command)]
pub async fn fix(
    ctx: Context<'_>,
    #[description = "URL to fix"] url: String,
) -> Result<(), Error> {
    let fixed = fixers::extract_links(&url);
    if fixed.is_empty() {
        ctx.say("No fixable social media link found.").await?;
        return Ok(());
    }
    let reply = fixed.iter().map(|l| l.fixed_url.clone()).collect::<Vec<_>>().join(" ");
    ctx.say(reply).await?;
    Ok(())
}
```

Register in `main.rs` commands vec.

---

## 11. Environment Variables

Add to `.env.example`:
```env
# Social media embed fixer
FXTWITTER_API_BASE=https://api.fxtwitter.com
FXBSKY_API_BASE=https://api.fxbsky.app
LINK_FIX_TIMEOUT_MS=3000
```

All fixer domains are hardcoded as constants (public services), but could be made configurable later.

---

## 12. Error Handling & Resilience

| Scenario | Behavior |
|----------|----------|
| FxEmbed API timeout (>3s) | Fallback to base URL (no translation) |
| FxEmbed returns non-200 | Fallback to base URL |
| `tweet.lang` missing | Fallback to base URL (don't guess) |
| `suppress_embeds` fails (no perms) | Log warning, still send reply |
| Regex matches but rewrite fails | Skip that link, process others |
| All links fail | Do not send reply, return `Ok(false)` |
| `reqwest` not available | Compile-time dependency (acceptable) |

---

## 13. Performance Considerations

- **Concurrent API calls:** If a message has multiple Twitter/X + Bluesky links, fire `reqwest` calls concurrently with `futures::join!` or `tokio::join!`.
- **Caching:** No persistent cache needed for v1. FxEmbed API is fast (~50-150ms). If rate limits appear, add an in-memory LRU cache (`cached` crate) mapping tweet ID → lang.
- **Timeout:** 3 seconds max per API call.

---

## 14. Testing Strategy

- **Unit tests** in each `fixers/*.rs` for regex matching and URL rewriting.
- **Mock tests** for FxEmbed API using `reqwest::Client` with a mock server or `wiremock`.
- **Integration:** Deploy to staging, post real links, verify embeds.

---

## 15. Implementation Phases (for the PR)

| Phase | Scope | Files |
|-------|-------|-------|
| **1. Foundation** | Add deps, create `fixers/` + `models/` modules, implement regex + rewrite for all platforms (no API calls yet) | `Cargo.toml`, `fixers/*.rs`, `models/*.rs` |
| **2. Translation** | Add `reqwest`, implement FxEmbed API calls for Twitter + Bluesky, lang-gated logic | `fixers/twitter.rs`, `fixers/bluesky.rs` |
| **3. Handler** | Wire into `bot/handlers/mod.rs`, suppress embeds, reply formatting | `bot/handlers/social_link.rs`, `bot/handlers/mod.rs` |
| **4. Edit tracking** | SQLite table, `MessageUpdate` event handling | `memory.rs`, `bot/handlers/mod.rs` |
| **5. Slash command** | `/fix` command | `bot/commands/fix.rs` |
| **6. Polish** | Stats integration, env vars, README update | `bot/commands/stats.rs`, `.env.example`, `README.md` |

---

## 16. Open Questions

1. **Bluesky API base:** Should we use `api.fxbsky.app` or `api.fxtwitter.com` with a different path? Needs verification during implementation.
2. **Edit tracking deletion:** If user removes all links, should the bot delete its reply or edit to "(links removed)"?
3. **TikTok reliability:** `tnktok.com` is new — should we add a health check or simply accept breakage?

---

**Ready for your review.** Approve this plan and I'll cut the implementation PR.
