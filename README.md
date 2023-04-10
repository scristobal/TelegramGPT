# Chatlyze 🤖💬

A Telegram bot that uses OpenAI's chat completion API in the backend. That is the same backend as ChatGPT.

## Config

Set your [Telegram bot token](https://core.telegram.org/bots/features#creating-a-new-bot) with `TELOXIDE_TOKEN` and your [OpenAI API key](https://platform.openai.com/account/api-keys) with `OPENAI_API_KEY`. Access to `gpt-4` is required.

## Run it locally

Rename `.env.template` to `.env` and fill in with your secrets. Launch the bot with `cargo run`. If you don't have rust, you need to [install Rust](https://www.rust-lang.org/tools/install)

### Deploy on cloud

The easiest way is to deploy it in fly.io, but you can also use the `Dockerfile` to deploy anywhere.
