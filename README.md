# some-roast-bot

A sarcastic Discord bot that roasts users while answering their questions. It talks to any OpenAI-compatible endpoint (by default [OpenRouter](https://openrouter.ai)) with a multimodal model, and searches the web through the [Exa MCP](https://exa.ai) server. All responses are in French.

---

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [Architecture](#architecture)
- [Deployment](#deployment)
- [License](#license)

---

## Features

### Slash Command

- **`/ask <question>`** — Asks the AI a question. The agent reads the last 15 messages of the channel, searches the web via Exa MCP when needed, then answers with a tease that also contains the actual answer.
- **`/research <question>`** — Longer answer in markdown with sources, after up to three web searches.
- **`/fix <url>`** — Manually fix a social media link and get the embed-friendly version back.
- **`/stats`** — Show usage statistics for all bot features (roasts, social links, etc.).

### Passive Triggers (no mention required)

- **Social Media Embed Fixer** — Automatically detects Twitter/X, Bluesky, Instagram, Reddit, and TikTok links. Suppresses the original broken/paywalled embed and replies with a fixed embed-friendly URL. For Twitter/X and Bluesky, it intelligently translates non-English/French posts to French via the FxEmbed API.
- **Microsoft/Windows Auto-Roast** — Whenever anyone mentions "Microsoft" or "Windows", the bot automatically searches for the latest Microsoft fails/bugs and roasts them. Uses the memory database to avoid repeating topics. Refers to Microsoft as "Microslop" and Windows as "Windaube".
- **"Is this true?" Detector** — Detects phrases like "is this true?" or "is that true?", reads recent channel messages, and judges whether the claim is true, false, or nonsense — roast-style.
- **Quoi → feur** — A message ending with "quoi" gets "-feur".

### Mention-Required Trigger

- **Roast** — Mention the bot. It reads the last 20 messages, their images and link previews, and roasts what the trigger asks for: the message it replies to (even with just "."), an explicit target ("roast @X", "roast cette image"), or else the most roastable subject of the conversation.

---

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- A [Discord bot token](https://discord.com/developers/applications)
- An OpenAI-compatible API key (the default uses [OpenRouter](https://openrouter.ai)) and a model that accepts images

---

## Installation

```bash
# Clone the repository
git clone https://github.com/lmandrelli/some-roast-bot.git
cd some-roast-bot

# Build the project
cargo build --release
```

Or with Docker:

```bash
docker compose up --build
```

---

## Configuration

Copy the example environment file and fill in your credentials:

```bash
cp .env.example .env
```

| Variable          | Description                                                       | Default                            |
| ----------------- | ----------------------------------------------------------------- | ---------------------------------- |
| `DISCORD_TOKEN`   | Discord bot token                                                 | *(required)*                       |
| `OPENAI_API_KEY`  | API key of the OpenAI-compatible endpoint                        | *(required)*                       |
| `OPENAI_BASE_URL` | Base URL of the OpenAI-compatible endpoint                       | `https://openrouter.ai/api/v1`     |
| `MODEL_NAME`      | Model (or OpenRouter `@preset/<slug>`). Must accept images: they are sent with the transcript | *(required)*                       |
| `STATUS_MESSAGE`  | Discord custom status                                             | `don't try to talk about Microslop` |
| `MAGIC_WORD`      | Word that switches every agent to a friendly easter-egg mode     | `magic`                            |
| `MEMORY_DB_PATH`  | SQLite memory database                                            | `data/memory.db`                   |
| `DATABASE_URL`    | Postgres URL (`postgres://user:pass@host:5432/db`). When set, replaces SQLite and `MEMORY_DB_PATH` is ignored | *(unset: SQLite)* |

`PROD` and `VISION_MODEL_NAME` are no longer read; the bot logs a warning when they are still set.

> **Note:** The bot reads these variables at startup, from the environment or a `.env` file ([`dotenvy`](https://crates.io/crates/dotenvy)). Never commit your `.env` file.

---

## Usage

```bash
# Run from source
cargo run --release
```

Once running, invite the bot to your server using the OAuth2 URL, then:

- Post a Twitter/X, Bluesky, Instagram, Reddit, or TikTok link — the bot will automatically fix it
- Type `/fix <url>` to manually fix a social media link
- Type `/ask` in any channel the bot has access to
- Mention the bot to trigger a roast (in a reply, or with another user, to point it at someone)
- Just mention "Microsoft" or "Windows" in any message and watch it react

---

## Architecture

```
src/
├── main.rs              # Startup wiring: env, framework, bot launch
├── config.rs            # Environment variables
├── error.rs             # Error types + Discord-friendly LLM error replies
├── db/
│   ├── mod.rs           # MemoryRepository trait
│   └── sql.rs           # sqlx repository (SQLite or Postgres)
├── models/
│   └── fxtwitter.rs     # FxEmbed API response structs
├── fixers/
│   ├── mod.rs           # Link extraction orchestrator
│   ├── twitter.rs       # Twitter/X detection + smart translation
│   ├── bluesky.rs       # Bluesky detection + smart translation
│   ├── instagram.rs     # Instagram → embed providers, kkinstagram as last resort
│   ├── reddit.rs        # Reddit → rxddit.com
│   └── tiktok.rs        # TikTok → tnktok.com
├── agents/
│   ├── mod.rs           # Re-exports
│   ├── llm/mod.rs       # LlmService: model + Exa MCP tools
│   ├── preambles.rs     # System prompts + magic-word override
│   ├── prompt.rs        # One user message: instruction, transcript, images
│   ├── images.rs        # Downloads and checks the context images
│   ├── links.rs         # Previews links Discord did not preview (trigger, replied message)
│   ├── eval.rs          # Prompt evaluation on made-up conversations (ignored test)
│   ├── ask.rs           # /ask agent
│   ├── research.rs      # /research agent
│   └── roast/
│       ├── mod.rs       # Reply hint and JSON retry loop
│       ├── channel.rs   # Intent-first roast on mention
│       ├── truth.rs     # Judge "is this true?" claims
│       └── microsoft.rs # Auto-roast on Microsoft/Windows mentions (web search + topic memory)
└── bot/
    ├── mod.rs           # Shared types (Data, Error, Context)
    ├── pipeline.rs      # Runs the first matching handler by priority
    ├── utils.rs         # Mention sanitizing for replies
    ├── context/         # Channel transcript fetch + formatting
    ├── commands/        # /ask, /research, /fix, /stats
    └── handlers/
        ├── mod.rs       # Event handler + pipeline setup
        ├── social_link.rs # Social media link fixer
        ├── quoi.rs      # Quoi → feur
        ├── microsoft.rs # Microsoft/Windows keyword detector
        ├── truth.rs     # Truth-check handler
        └── channel.rs   # Roast on mention
```

**Key components:**

- **`poise` + `serenity`** — Discord bot framework and gateway client.
- **`rig-core`** — Agent builder that composes the LLM model with MCP tools.
- **`rmcp`** — MCP client that connects to Exa's search server and exposes web-search tools to the agent.
- **`sqlx`** — Memory database (Microsoft topics, stats) on SQLite by default or Postgres with `DATABASE_URL`. Migrations per backend live in `migrations/sqlite` and `migrations/postgres` and run at startup.

**Design notes:**

- **Stateless agents** — Each request cold-starts a new MCP connection and rig agent. Everything is dropped when the call completes, so there is no shared state or long-lived connection to manage.
- **One call per roast** — The model gets the transcript and the images (at most 10, trigger and reply target first) in the same prompt, each image right after the message that posted it. Text, images and pages are marked as untrusted data.
- **Transcript** — Messages are numbered with their age ("il y a 12 min"), replies point to the number they answer, the trigger quotes the message it replies to, and the bot's own messages are marked so it does not repeat a joke. The current date is given for news and fact checks.
- **Link previews** — Discord often delivers the trigger before its embed. For the trigger and the message it replies to, the bot reads the title and description of up to 3 links itself (public hosts only, 5 s, 512 KB). The model can still open other pages with Exa.
- **Prompts** — In French (`src/agents/preambles.rs`), with one shared persona: sharp French humour, one precise joke grounded in the conversation, no invented facts, no protected traits. `cargo test eval -- --ignored --nocapture` replays made-up conversations against the configured model to compare prompt changes.
- **Priority-based dispatch** — Event handlers are checked in order: social media links > quoi > Microsoft keywords > truth questions > roast on mention.
- **Smart translation** — Twitter/X and Bluesky links are translated to French only when the detected language is neither English nor French, using the FxEmbed API.
- **Separation of concerns** — `agents/` contains pure async LLM logic (no Discord awareness). `bot/` handles all Discord interaction. `fixers/` handles link detection and rewriting independently.

---

## Deployment

Pushing a version tag builds and publishes the image:

```bash
git tag v1.0.0
git push origin v1.0.0
```

GitHub Actions builds the Docker image and pushes `ghcr.io/lmandrelli/some-roast-bot` tagged `1.0.0`, `1.0` and `latest`. Deployments pick up new tags on their side (e.g. Argo CD Image Updater on `X.Y.Z` tags); the CI does not touch any infrastructure repository.

Run a single instance per Discord token, or every message gets answered twice. Without `DATABASE_URL`, keep `MEMORY_DB_PATH` on a persistent volume (`/data` in `docker-compose.yml`).

---

## License

This project is licensed under the [MIT License](LICENSE).
