use crate::openai_client::reply;
use async_openai::{
    types::{ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs, Role},
    Client,
};
use dptree::case;
use std::time::{Duration, Instant};
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
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
use tracing::{error, instrument};
use uuid::Uuid;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "Starts a new conversation, ie. wipe bot's memory")]
    Start,
    #[command(description = "Send a message to the bot, required only on groups")]
    Chat { text: String },
}

#[derive(Debug, Clone, Default)]
pub struct State {
    chat_history: Vec<ChatCompletionRequestMessage>,
}

pub fn schema() -> UpdateHandler<anyhow::Error> {
    let cmd_handler = filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(reset))
        .branch(case![Command::Chat { text }].endpoint(chat));

    let msg_handler = Update::filter_message()
        .branch(cmd_handler)
        .filter_map(|message: Message| {
            if message.chat.is_private() {
                message.text().map(|text| text.to_string())
            } else {
                None
            }
        })
        .endpoint(chat);

    dialogue::enter::<Update, InMemStorage<State>, State, _>().branch(msg_handler)
}

type InMemDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), anyhow::Error>;

async fn reset(bot: Bot, dialogue: InMemDialogue, message: Message) -> HandlerResult {
    bot.send_chat_action(message.chat.id, teloxide::types::ChatAction::Typing)
        .await?;

    dialogue.update(State::default()).await?;

    bot.send_message(message.chat.id, "`Starting a new conversation`")
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

#[instrument]
async fn chat(
    bot: Bot,
    dialogue: InMemDialogue,
    client: Client,
    text: String,
    message: Message,
) -> HandlerResult {
    let username = message.from().and_then(|user| user.username.clone());

    bot.send_chat_action(message.chat.id, ChatAction::Typing)
        .await?;

    let State { mut chat_history } = dialogue.get().await?.unwrap_or_default();

    let new_message = ChatCompletionRequestMessage {
        role: Role::User,
        content: text,
        name: username,
    };

    chat_history.push(new_message);

    let response = reply(&chat_history, Some(client), None, None).await;

    match response {
        Err(e) => {
            let error_id = Uuid::new_v4().simple().to_string();

            error!(error_id, ?e);

            bot.send_message(
                message.chat.id,
                format!("there was an error processing your request, you can use this ID to track the issue `{}`", error_id),
            ).parse_mode(ParseMode::MarkdownV2)
            .await?;
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
                    bot.send_chat_action(message.chat.id, ChatAction::Typing)
                        .await?;

                    now = Instant::now();
                }
            }

            bot.send_message(message.chat.id, &full_text).await?;

            let bot_message = ChatCompletionRequestMessageArgs::default()
                .role(Role::Assistant)
                .content(&full_text)
                .build()
                .unwrap();

            chat_history.push(bot_message);

            dialogue.update(State { chat_history }).await.unwrap();
        }
    };

    Ok(())
}
