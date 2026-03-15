# some-roast-bot

A sarcastic Discord bot that roasts users while answering their questions. Powered by an AI LLM (currently [Kimi K2.5](https://kimi.ai) served by [Chutes AI](https://chutes.ai)) with real-time web search via the [Exa MCP](https://exa.ai) server. All responses are in French.

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

- **`/ask <question>`** — Asks the AI a question. The agent searches the web via Exa MCP, then responds with a sarcastic roast that also contains the actual answer.

### Passive Triggers (no mention required)

- **Microsoft/Windows Auto-Roast** — Whenever anyone mentions "Microsoft" or "Windows", the bot automatically searches for the latest Microsoft fails/bugs and roasts them. Uses a SQLite memory to avoid repeating topics. Refers to Microsoft as "Microslop" and Windows as "Windaube".
- **"Is this true?" Detector** — Detects phrases like "is this true?" or "is that true?", reads recent channel messages, and judges whether the claim is true, false, or nonsense — roast-style.

### Mention-Required Triggers

- **Reply Roast** — Tag the bot inside a reply to another message, and it settles the argument by roasting whoever is wrong.
- **Targeted User Roast** — Mention the bot alongside another user, and it fetches that user's recent messages and roasts them based on what they said.
- **Channel Roast** — Mention the bot alone, and it reads recent messages, picks whoever "deserves it most", and roasts them.

---

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)
- A [Discord bot token](https://discord.com/developers/applications)
- An OpenAI-compatible API key (the default uses [Chutes AI](https://chutes.ai))

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

| Variable         | Description                                             | Default                         |
| ---------------- | ------------------------------------------------------- | ------------------------------- |
| `DISCORD_TOKEN`  | Your Discord bot token                                  | *(required)*                    |
| `OPENAI_API_KEY` | API key for the LLM provider (Chutes or OpenAI-compat.) | *(required)*                    |
| `OPENAI_BASE_URL`| Base URL of the OpenAI-compatible API endpoint          | `https://llm.chutes.ai/v1`     |
| `MODEL_NAME`     | LLM model identifier                                   | `moonshotai/Kimi-K2.5-TEE`     |
| `PROD`           | Production flag (`1` = prod, `0` = dev status display)  | `1`                             |
| `MEMORY_DB_PATH` | Path to the SQLite memory database                      | `data/memory.db`                |

> **Note:** The bot reads these variables at startup via [`dotenv`](https://crates.io/crates/dotenv). Never commit your `.env` file.

---

## Usage

```bash
# Run from source
cargo run --release
```

Once running, invite the bot to your server using the OAuth2 URL, then:

- Type `/ask` in any channel the bot has access to
- Mention the bot (with or without tagging another user) to trigger a roast
- Reply to a message and tag the bot to have it settle an argument
- Just mention "Microsoft" or "Windows" in any message and watch it react

---

## Architecture

```
src/
├── main.rs              # Startup wiring: env, framework, bot launch
├── memory.rs            # SQLite-backed topic deduplication memory
├── agents/
│   ├── mod.rs           # Re-exports + model_name() helper
│   ├── ask.rs           # /ask agent: MCP + LLM with web search
│   └── roast/
│       ├── mod.rs       # Shared call_model() helper
│       ├── channel.rs   # Roast based on recent channel messages
│       ├── user.rs      # Roast a specific tagged user
│       ├── reply.rs     # Settle an argument between two users
│       ├── truth.rs     # Judge "is this true?" claims
│       └── microsoft.rs # Auto-roast on Microsoft/Windows mentions (MCP + memory)
└── bot/
    ├── mod.rs           # Shared types (Data, Error, Context)
    ├── commands/
    │   ├── mod.rs       # Re-exports
    │   └── ask.rs       # /ask slash command handler
    └── handlers/
        ├── mod.rs       # Event handler + priority dispatch logic
        ├── channel.rs   # Channel roast handler
        ├── user.rs      # Targeted user roast handler
        ├── reply.rs     # Reply-chain roast handler
        ├── truth.rs     # Truth-check handler
        └── microsoft.rs # Microsoft/Windows keyword detector
```

**Key components:**

- **`poise` + `serenity`** — Discord bot framework and gateway client.
- **`rig-core`** — Agent builder that composes the LLM model with MCP tools.
- **`rmcp`** — MCP client that connects to Exa's search server and exposes web-search tools to the agent.
- **`rusqlite`** — SQLite database for topic deduplication memory (Microsoft roast).

**Design notes:**

- **Stateless agents** — Each request cold-starts a new MCP connection and rig agent. Everything is dropped when the call completes, so there is no shared state or long-lived connection to manage.
- **Priority-based dispatch** — Event handlers are checked in order: Microsoft keywords > truth questions > reply roast > user roast > channel roast.
- **Separation of concerns** — `agents/` contains pure async LLM logic (no Discord awareness). `bot/` handles all Discord interaction. The AI logic could be reused outside of Discord.

---

## Deployment

The project includes a CI/CD pipeline that automatically builds a Docker image and pushes it to GitHub Container Registry (GHCR) when a version tag is pushed. A Portainer stack then pulls and runs the image on your server.

### How it works

1. You push a tag like `v1.0.0` to GitHub
2. GitHub Actions builds the Docker image and pushes it to `ghcr.io/lmandrelli/some-roast-bot`
3. Portainer detects the new image and redeploys the container via its stack webhook

### Initial Portainer setup (one-time)

1. **Create the stack** in Portainer:
   - Go to **Stacks > Add stack**
   - Choose **Web editor** and paste the content of `docker-compose.yml`
   - Under **Environment variables**, add the variables from `.env.example` with your actual values. Alternatively, upload a `stack.env` file.

2. **Enable the stack webhook** for automatic redeployment:
   - After creating the stack, go to the stack's settings
   - Enable **Stack webhook**
   - Copy the generated webhook URL (it looks like `https://<portainer-host>/api/stacks/webhooks/<uuid>`)

3. **Trigger redeployment** after a new image is pushed:
   - Since Portainer is on your local network and not exposed to the internet, you can trigger the webhook locally with:
     ```bash
     curl -X POST https://192.168.7.102:9443/api/stacks/webhooks/<uuid> -k
     ```
   - Alternatively, set up a cron job or a small script on a machine on your local network to poll and trigger it periodically.

### Publishing a new version

```bash
git tag v1.0.0
git push origin v1.0.0
```

This triggers the GitHub Actions workflow, which builds and pushes the image tagged as `1.0.0`, `1.0`, and `latest`.

---

## License

This project is licensed under the [MIT License](LICENSE).
