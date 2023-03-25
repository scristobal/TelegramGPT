use crate::telegram_bot;
use async_openai::{self, types::ChatCompletionRequestMessageArgs};

impl From<telegram_bot::Role> for async_openai::types::Role {
    fn from(val: telegram_bot::Role) -> Self {
        match val {
            telegram_bot::Role::System => async_openai::types::Role::System,
            telegram_bot::Role::User => async_openai::types::Role::User,
            telegram_bot::Role::Assistant => async_openai::types::Role::Assistant,
        }
    }
}

impl From<telegram_bot::BotMessage> for async_openai::types::ChatCompletionRequestMessage {
    fn from(value: telegram_bot::BotMessage) -> Self {
        let mut req = ChatCompletionRequestMessageArgs::default();

        req.role(value.role).content(&value.content);

        if let Some(name) = &value.name {
            req.name(name);
        }

        req.build().unwrap()
    }
}
