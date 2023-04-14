use crate::openai_client::reply;
use async_openai::{
    types::{ChatCompletionRequestMessage, ChatCompletionResponseStream, Role},
    Client,
};
use dptree::case;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use teloxide::{
    dispatching::{
        dialogue::{self, ErasedStorage},
        UpdateFilterExt, UpdateHandler,
    },
    filter_command,
    payloads::SendMessageSetters,
    prelude::Dialogue,
    requests::Requester,
    types::{ChatAction, Message, Update},
    utils::command::BotCommands,
    Bot, RequestError,
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

type Storage = ErasedStorage<State>;
type Dialog = Dialogue<State, ErasedStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync>> {
    let cmd_handler = filter_command::<Command, _>().branch(case![Command::Start].endpoint(reset));

    let msg_handler = Update::filter_message().branch(cmd_handler).endpoint(chat);

    dialogue::enter::<Update, Storage, State, _>().branch(msg_handler)
}

async fn reset(bot: Bot, dialogue: Dialog, message: Message) -> HandlerResult {
    let chat_id = message.chat.id;

    bot.send_chat_action(chat_id, teloxide::types::ChatAction::Typing)
        .await?;

    dialogue.reset().await?;

    let text = "Starting a new conversation. Reply this message to begin.";
    send_reply(&bot, &message, text).await?;

    Ok(())
}

async fn chat(bot: Bot, dialogue: Dialog, client: Client, message: Message) -> HandlerResult {
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
        Err(error) => send_error(&bot, &message, error.into()).await?,

        Ok(mut response_stream) => {
            let botname = bot.get_me().await?.user.username;

            let bot_message = send_stream(&bot, &message, &mut response_stream).await?;

            let bot_request = ChatCompletionRequestMessage {
                role: Role::Assistant,
                content: bot_message.text().unwrap_or("").to_string(),
                name: botname,
            };

            chat_history.push(bot_request);

            dialogue.update(State { chat_history }).await?;
            bot_message
        }
    };

    Ok(())
}

async fn send_reply(bot: &Bot, message: &Message, text: &str) -> Result<Message, RequestError> {
    Ok(if !message.chat.is_private() {
        bot.send_message(message.chat.id, text)
            .reply_to_message_id(message.id)
            .await?
    } else {
        bot.send_message(message.chat.id, text).await?
    })
}

async fn send_error(
    bot: &Bot,
    message: &Message,
    error: anyhow::Error,
) -> Result<Message, RequestError> {
    let error_id = Uuid::new_v4().simple().to_string();

    error!(error_id, ?error);
    let error_text = format!(
        "there was an error processing your request, you can use this ID to track the issue `{}`",
        error_id
    );

    send_reply(bot, message, &error_text).await
}

async fn send_stream(
    bot: &Bot,
    message: &Message,
    response_stream: &mut ChatCompletionResponseStream,
) -> Result<Message, RequestError> {
    let mut full_text = "".to_string();

    let mut now = Instant::now();

    while let Some(partial_response) = response_stream.next().await {
        match partial_response {
            Ok(partial_response) => {
                // info!(?partial_response); // somehow partial_response.usage is always None :|

                let Some(delta_text) = partial_response
                    .choices
                    .first()
                    .and_then(|choice| choice.delta.content.as_ref()) else {continue;};

                full_text.push_str(delta_text);

                let elapsed_time = now.elapsed();

                if elapsed_time > Duration::from_secs(1) {
                    bot.send_chat_action(message.chat.id, ChatAction::Typing)
                        .await?;
                    now = Instant::now();
                }
            }
            Err(error) => {
                send_error(bot, message, error.into()).await?;
                break;
            }
        }
    }

    send_reply(bot, message, &full_text).await
}
