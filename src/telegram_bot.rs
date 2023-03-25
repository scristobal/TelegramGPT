use std::fmt::Display;

use crate::openai_client::{self, reply};

use serde::Serialize;
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateHandler,
    },
    filter_command,
    prelude::*,
    types::ParseMode,
    utils::{command::BotCommands, markdown::escape},
};
use tracing::{error, instrument};
use uuid::Uuid;

use dptree::case;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
#[derive(Debug)]
pub enum Command {
    #[command(description = "Give the current state of the bot")]
    State,
    #[command(description = "Clear chat history")]
    Reset,
    #[command(description = "The bot will stop listening to messages")]
    Mute,
    #[command(description = "The bot will start listening to messagess")]
    Listen,
    #[command(description = "Make a summary of the group activity")]
    Summarize,
    #[command(description = "Chat with the bot")]
    Chat { text: String },
}

type GroupHistory = Vec<Message>;

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Serialize, Clone)]
pub struct BotMessage {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

type BotHistory = Vec<BotMessage>;

#[derive(Debug, Clone, Default)]
pub struct History {
    group_history: GroupHistory,
    bot_history: BotHistory,
}

#[derive(Debug, Clone)]
pub enum State {
    Offline,
    Online(History),
}

impl Default for State {
    fn default() -> Self {
        Self::Online(History::default())
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Offline => write!(f, "`sate ` ❌"),
            State::Online(_) => write!(f, "`state` ✅"),
        }
    }
}

#[instrument]
pub fn schema() -> UpdateHandler<anyhow::Error> {
    let cmd_handler = filter_command::<Command, _>()
        .branch(case![Command::State].endpoint(state))
        .branch(case![State::Offline].branch(case![Command::Listen].endpoint(listen)))
        .branch(
            case![State::Online(msgs)]
                .branch(case![Command::Summarize].endpoint(sumarize))
                .branch(case![Command::Reset].endpoint(reset))
                .branch(case![Command::Mute].endpoint(mute))
                .branch(case![Command::Chat { text }].endpoint(chat)),
        );

    let msg_handler = Update::filter_message()
        .branch(cmd_handler)
        .branch(case![State::Offline].endpoint(muted))
        .branch(case![State::Online(msgs)].endpoint(incoming_message))
        .endpoint(invalid);

    dialogue::enter::<Update, InMemStorage<State>, State, _>().branch(msg_handler)
}

type InMemDialogue = Dialogue<State, InMemStorage<State>>;

type HandlerResult = Result<(), anyhow::Error>;

async fn state(bot: Bot, dialogue: InMemDialogue, message: Message) -> HandlerResult {
    let state = dialogue.get().await?;

    let reply_txt = match state {
        None => "No active conversation".to_string(),
        Some(state) => format!("{}", state),
    };

    bot.send_message(message.chat.id, reply_txt)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

async fn listen(bot: Bot, dialogue: InMemDialogue, message: Message) -> HandlerResult {
    dialogue.update(State::default()).await?;

    bot.send_message(message.chat.id, State::default().to_string())
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

async fn mute(bot: Bot, dialogue: InMemDialogue, message: Message) -> HandlerResult {
    dialogue.update(State::Offline).await?;

    bot.send_message(message.chat.id, State::Offline.to_string())
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

async fn invalid(bot: Bot, dialogue: InMemDialogue, message: Message) -> HandlerResult {
    dialogue.exit().await?;

    let error_id = Uuid::new_v4().simple().to_string();

    error!(error_id);

    bot.send_message(
                message.chat.id,
                format!("there was an error processing your request, you can use this ID to track the issue `{}`", error_id),
            ).parse_mode(ParseMode::MarkdownV2)
            .await?;

    Ok(())
}

async fn sumarize(bot: Bot, dialogue: InMemDialogue, message: Message) -> HandlerResult {
    let state = dialogue.get().await?;

    let messages = match state {
        None => {
            bot.send_message(message.chat.id, "No active conversation")
                .await?;
            return Ok(());
        }
        Some(State::Offline) => {
            bot.send_message(message.chat.id, "The bot is offline")
                .await?;
            return Ok(());
        }
        Some(State::Online(msgs)) => msgs,
    };

    let openai_response = openai_client::sumarize(&messages.group_history).await;

    match openai_response {
        Err(e) => {
            let error_id = Uuid::new_v4().simple().to_string();

            error!(error_id, ?e);

            bot.send_message(
                message.chat.id,
                format!("there was an error processing your request, you can use this ID to track the issue `{}`", error_id),
            ).parse_mode(ParseMode::MarkdownV2)
            .await?;
        }
        Ok(responses) => {
            let mut reply_txt = String::new();

            for choice in responses.choices {
                let result = choice.message.content;

                reply_txt.push_str(&result);
            }

            bot.send_message(message.chat.id, escape(&reply_txt))
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
    }

    Ok(())
}

async fn reset(bot: Bot, dialogue: InMemDialogue, message: Message) -> HandlerResult {
    dialogue.update(State::default()).await?;

    bot.send_message(message.chat.id, State::default().to_string())
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

async fn muted() -> HandlerResult {
    // if the bot is muted do nothing
    Ok(())
}

async fn incoming_message(
    dialogue: InMemDialogue,
    new_message: Message,
    mut history: History,
) -> HandlerResult {
    history.group_history.push(new_message);
    dialogue.update(State::Online(history)).await?;

    Ok(())
}

async fn chat(
    bot: Bot,
    dialogue: InMemDialogue,
    text: String,
    message: Message,
    mut history: History,
) -> HandlerResult {
    let username = message.from().and_then(|user| user.username.clone());

    history.bot_history.push(BotMessage {
        role: Role::User,
        content: text,
        name: username,
    });

    let results = reply(
        &history
            .bot_history
            .clone()
            .into_iter()
            .map(|m| m.into())
            .collect::<Vec<_>>(),
        None,
        None,
    )
    .await;

    match results {
        Err(e) => {
            let error_id = Uuid::new_v4().simple().to_string();

            error!(error_id, ?e);

            bot.send_message(
                message.chat.id,
                format!("there was an error processing your request, you can use this ID to track the issue `{}`", error_id),
            ).parse_mode(ParseMode::MarkdownV2)
            .await?;
        }
        Ok(results) => {
            let botname = &bot.get_me().await?.username;

            let mut reply_txt = String::new();

            for choice in results.choices {
                let result = choice.message.content;

                reply_txt.push_str(&result);

                history.bot_history.push(BotMessage {
                    role: Role::Assistant,
                    content: result,
                    name: botname.clone(),
                });
            }

            dialogue.update(State::Online(history)).await.unwrap();

            reply_txt = escape(&reply_txt);

            if let Some(usage) = results.usage {
                reply_txt.push_str(&format!(
                    "\n\n`usage {} tokens = {} prompt + {} completion`",
                    usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
                ));

                if usage.total_tokens > 6000 {
                    reply_txt.push_str("\n`Reaching 8k limit, consider running /reset soon`")
                }
            }

            bot.send_message(message.chat.id, &reply_txt)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
    };

    Ok(())
}
