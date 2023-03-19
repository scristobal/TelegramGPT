use std::fmt::Display;

use crate::openai_client;

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
    #[command()]
    State,
    Reset,
    Mute,
    Listen,
    Sumarize,
}

#[derive(Debug, Default, Clone)]
pub enum State {
    #[default]
    Muted,
    Listening(Vec<Message>),
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Muted => write!(f, "Sate: muted"),
            State::Listening(msgs) => {
                f.write_fmt(format_args!("State:chatting({} msgs)", msgs.len()))
            }
        }
    }
}

#[instrument]
pub fn schema() -> UpdateHandler<anyhow::Error> {
    let cmd_handler = filter_command::<Command, _>()
        .branch(case![Command::State].endpoint(state))
        .branch(case![State::Muted].branch(case![Command::Listen].endpoint(listen)))
        .branch(
            case![State::Listening(msgs)]
                .branch(case![Command::Sumarize].endpoint(sumarize))
                .branch(case![Command::Reset].endpoint(reset))
                .branch(case![Command::Mute].endpoint(mute)),
        );

    let msg_handler = Update::filter_message()
        .branch(cmd_handler)
        .branch(case![State::Muted].endpoint(muted))
        .branch(case![State::Listening(msgs)].endpoint(incoming_message))
        .endpoint(invalid);

    dialogue::enter::<Update, InMemStorage<State>, State, _>().branch(msg_handler)
}

type InMemDialogue = Dialogue<State, InMemStorage<State>>;

type HandlerResult = Result<(), anyhow::Error>;

async fn state(bot: Bot, dialogue: InMemDialogue, msg: Message) -> HandlerResult {
    let state = dialogue.get().await?;

    let reply_txt = match state {
        None => "No active conversation".to_string(),
        Some(state) => format!("State: {:}", state),
    };

    bot.send_message(msg.chat.id, reply_txt).await?;

    Ok(())
}

async fn listen(bot: Bot, dialogue: InMemDialogue, msg: Message) -> HandlerResult {
    dialogue.update(State::Listening(vec![])).await?;

    bot.send_message(msg.chat.id, "`state:listening`")
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

async fn mute(bot: Bot, dialogue: InMemDialogue, msg: Message) -> HandlerResult {
    dialogue.update(State::Muted).await?;

    bot.send_message(msg.chat.id, "`state:muted`")
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

async fn invalid(bot: Bot, dialogue: InMemDialogue, msg: Message) -> HandlerResult {
    dialogue.exit().await?;

    let error_id = Uuid::new_v4().simple().to_string();

    error!(error_id);

    bot.send_message(
                msg.chat.id,
                format!("there was an error processing your request, you can use this ID to track the issue `{}`", error_id),
            ).parse_mode(ParseMode::MarkdownV2)
            .await?;

    Ok(())
}

async fn sumarize(bot: Bot, dialogue: InMemDialogue, msg: Message) -> HandlerResult {
    let state = dialogue.get().await?;

    let messages = match state {
        None => {
            bot.send_message(msg.chat.id, "No active conversation")
                .await?;
            return Ok(());
        }
        Some(State::Muted) => {
            bot.send_message(msg.chat.id, "The bot is muted").await?;
            return Ok(());
        }
        Some(State::Listening(msgs)) => msgs,
    };

    let openai_response = openai_client::sumarize(&messages).await;

    match openai_response {
        Err(e) => {
            let error_id = Uuid::new_v4().simple().to_string();

            error!(error_id, ?e);

            bot.send_message(
                msg.chat.id,
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

            bot.send_message(msg.chat.id, escape(&reply_txt))
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
    }

    Ok(())
}

async fn reset(bot: Bot, dialogue: InMemDialogue, msg: Message) -> HandlerResult {
    dialogue.update(State::Listening(vec![])).await?;

    bot.send_message(msg.chat.id, "`status:listening:clear-history`")
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
    mut messages: Vec<Message>,
) -> HandlerResult {
    messages.push(new_message);
    dialogue.update(State::Listening(messages)).await?;

    Ok(())
}
