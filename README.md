# TelegramGPT 🤖💬

Like ChatGPT but in Telegram

A Telegram bot that uses OpenAI's chat completion API in the backend. That is the same backend as ChatGPT.

## Config

### Telegram and OpenAI tokens

Set your [Telegram bot token](https://core.telegram.org/bots/features#creating-a-new-bot) with `TELOXIDE_TOKEN` and your [OpenAI API key](https://platform.openai.com/account/api-keys) with `OPENAI_API_KEY`. Access to `gpt-4` is required.

### Storage (optional)

Optionally you can store the dialogs using a Redis or SQLite. This prevents your conversations to wipe when the bot is restarted.

To setup Redis use `REDIS_URL` and to setup SQLite use `SQLITE_FILE`. Redis has priority over SQLite. If none is set memory storage is used.

## Run

### Locally

Rename `.env.template` to `.env` and fill in with your secrets. Launch the bot with `cargo run`. If you don't have rust, you need to [install Rust](https://www.rust-lang.org/tools/install).

### Docker

Arguably, the easiest way to get it running is to use Docker. This does not require rust to be installed, but you need [Docker engine](https://docs.docker.com/engine/install/) and (optionally) [docker compose](https://docs.docker.com/compose/install/).

### Cloud

Even simpler would be to deploy on cloud, but it might cost you money. A [fly.io](https://fly.io/docs/languages-and-frameworks/dockerfile/) config file is provided, you can also deploy a [Redis](https://fly.io/docs/reference/redis/) over there. Free tier works jut fine for a few users.

## Usage

The bot will answer to private messages as you would expect. On groups it will only answer to reply messages, it has no access to any other messages. Start a new conversation with `/start`.
