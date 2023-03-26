use crate::openai_client::{self, reply};
use dptree::case;
use serde::Serialize;
use std::fmt::Display;
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

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
#[derive(Debug)]
pub enum Command {
    #[command(description = "Keep the conversation going, the bot will keep context until /reset")]
    Chat { text: String },
    #[command(description = "Ask questions in the context of the group conversation")]
    Group { text: String },
    #[command(description = "Wipe chat from the bot's memory")]
    Reset,
}

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

type GroupHistory = Vec<Message>;

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
    let cmd_handler = filter_command::<Command, _>().branch(
        case![State::Online(msgs)]
            .branch(case![Command::Group { text }].endpoint(group))
            .branch(case![Command::Reset].endpoint(reset))
            .branch(case![Command::Chat { text }].endpoint(chat)),
    );

    let msg_handler = Update::filter_message()
        .branch(case![State::Offline].endpoint(do_nothing))
        .branch(cmd_handler)
        .branch(case![State::Online(msgs)].endpoint(record))
        .endpoint(do_nothing);

    dialogue::enter::<Update, InMemStorage<State>, State, _>().branch(msg_handler)
}

type InMemDialogue = Dialogue<State, InMemStorage<State>>;

type HandlerResult = Result<(), anyhow::Error>;

async fn group(bot: Bot, text: String, message: Message, history: History) -> HandlerResult {
    let openai_response = openai_client::group_question(&history.group_history, text).await;

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

async fn reset(
    bot: Bot,
    dialogue: InMemDialogue,
    message: Message,
    mut history: History,
) -> HandlerResult {
    history.bot_history = BotHistory::default();

    dialogue.update(State::Online(history)).await?;

    bot.send_message(message.chat.id, "`Bot chat history has been erased` ✅")
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

async fn do_nothing() -> HandlerResult {
    // if the bot is muted do nothing
    Ok(())
}

async fn record(
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
