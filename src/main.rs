use dotenv::dotenv;
use std::io::Result;
use telegram_gpt::{
    health_checker,
    telegram_bot::{schema, Command, State},
};
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, utils::command::BotCommands};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    match dotenv() {
        Ok(_) => info!("Loaded .env file"),
        Err(_) => info!("No .env file found. Falling back to environment variables"),
    }

    tokio::spawn(health_checker::run(([0, 0, 0, 0], 8080)));

    info!("Starting bot...");

    let bot = teloxide::Bot::from_env();

    bot.set_my_commands(Command::bot_commands())
        .send()
        .await
        .unwrap();

    let me = bot.get_me().await.unwrap().mention();

    info!("... {} started!", me);

    let storage = InMemStorage::<State>::new();

    let openai_client = async_openai::Client::new();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![storage, openai_client])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
