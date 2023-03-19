use chatlyze::{
    health_checker,
    telegram_bot::{schema, State},
};
use dotenv::dotenv;
use std::io::Result;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};
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

    let me = bot.get_me().await.unwrap().mention();

    info!("... {} started!", me);

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
