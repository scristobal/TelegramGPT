use dotenv::dotenv;
use std::{io::Result, sync::Arc};
use telegram_gpt::{
    health_checker,
    telegram_bot::{schema, Command, State},
};
use teloxide::{
    dispatching::dialogue::{
        serializer::{Bincode, Json},
        ErasedStorage, InMemStorage, RedisStorage, SqliteStorage, Storage,
    },
    prelude::*,
    utils::command::BotCommands,
};
use tracing::info;

type StateStorage = Arc<ErasedStorage<State>>;

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

    let redis_url = std::env::var("REDIS_URL");
    let sqlite_file = std::env::var("SQLITE_FILE");

    let storage: StateStorage = match (redis_url, sqlite_file) {
        (Ok(url), _) => RedisStorage::open(url, Bincode)
            .await
            .expect("Failed to connect to Redis")
            .erase(),
        (_, Ok(filename)) => SqliteStorage::open(&filename, Json)
            .await
            .expect("Failed to open sqlite storage")
            .erase(),
        (_, _) => InMemStorage::<State>::new().erase(),
    };

    let openai_client = async_openai::Client::new();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![storage, openai_client])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
