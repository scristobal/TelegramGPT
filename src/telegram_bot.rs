use crate::openai_client::reply;
use async_openai::{
    types::{ChatCompletionRequestMessage, Role},
    Client,
};
use dptree::case;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use teloxide::{
    dispatching::{
        dialogue::{self, serializer::Bincode, RedisStorage},
        UpdateFilterExt, UpdateHandler,
    },
    filter_command,
    payloads::SendMessageSetters,
    prelude::Dialogue,
    requests::Requester,
    types::{ChatAction, Message, ParseMode, Update},
    utils::command::BotCommands,
    Bot,
};
use tokio_stream::StreamExt;
use tracing::error;
use uuid::Uuid;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "Start a new conversation")]
    Start,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    chat_history: Vec<ChatCompletionRequestMessage>,
}

pub fn schema() -> UpdateHandler<anyhow::Error> {
    let cmd_handler = filter_command::<Command, _>().branch(case![Command::Start].endpoint(reset));

    let msg_handler = Update::filter_message().branch(cmd_handler).endpoint(chat);

    dialogue::enter::<Update, RedisStorage<Bincode>, State, _>().branch(msg_handler)
}

type RedisDialogue = Dialogue<State, RedisStorage<Bincode>>;
type HandlerResult = Result<(), anyhow::Error>;

async fn reset(bot: Bot, dialogue: RedisDialogue, message: Message) -> HandlerResult {
    let chat_id = message.chat.id;

    bot.send_chat_action(chat_id, teloxide::types::ChatAction::Typing)
        .await?;

    dialogue.reset().await?;

    let confirmation_text = "Starting a new conversation. Reply this message to begin.";

    if !message.chat.is_private() {
        bot.send_message(chat_id, confirmation_text)
            .reply_to_message_id(message.id)
            .await?;
    } else {
        bot.send_message(chat_id, confirmation_text).await?;
    }

    Ok(())
}

async fn chat(
    bot: Bot,
    dialogue: RedisDialogue,
    client: Client,
    message: Message,
) -> HandlerResult {
    let username = message.from().and_then(|user| user.username.clone());

    let chat_id = message.chat.id;

    bot.send_chat_action(chat_id, ChatAction::Typing).await?;

    let State { mut chat_history } = dialogue.get_or_default().await?;

    let new_message = ChatCompletionRequestMessage {
        role: Role::User,
        content: message.text().unwrap_or("").to_string(),
        name: username,
    };

    chat_history.push(new_message);

    let response = reply(&chat_history, Some(client), None, None).await;

    match response {
        Err(e) => {
            let error_id = Uuid::new_v4().simple().to_string();

            error!(error_id, ?e);
            let error_text = format!("there was an error processing your request, you can use this ID to track the issue `{}`", error_id);

            if !message.chat.is_private() {
                bot.send_message(chat_id, error_text)
                    .reply_to_message_id(message.id)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
            } else {
                bot.send_message(chat_id, error_text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
            }
        }
        Ok(mut response_stream) => {
            let mut full_text = "".to_string();

            let mut now = Instant::now();

            while let Some(partial_response) = response_stream.next().await {
                let partial_response = partial_response?;

                // info!(?partial_response); // somehow partial_response.usage is always None :|

                let Some(delta_text) = partial_response
                    .choices
                    .first()
                    .and_then(|choice| choice.delta.content.as_ref()) else {continue;};

                full_text.push_str(delta_text);

                let elapsed_time = now.elapsed();

                if elapsed_time > Duration::from_secs(1) {
                    bot.send_chat_action(chat_id, ChatAction::Typing).await?;

                    now = Instant::now();
                }
            }

            if !message.chat.is_private() {
                bot.send_message(chat_id, &full_text)
                    .reply_to_message_id(message.id)
                    .await?;
            } else {
                bot.send_message(chat_id, &full_text).await?;
            }

            let botname = bot.get_me().await?.user.username;

            let bot_message = ChatCompletionRequestMessage {
                role: Role::Assistant,
                content: full_text,
                name: botname,
            };

            chat_history.push(bot_message);

            dialogue.update(State { chat_history }).await.unwrap();
        }
    };

    Ok(())
}
