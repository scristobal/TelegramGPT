use async_openai::{
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, CreateChatCompletionRequestArgs, CreateChatCompletionResponse,
    },
    Client,
};
use teloxide::types::Message;
use tracing::instrument;

#[instrument]
pub async fn sumarize(messages: &[Message]) -> Result<CreateChatCompletionResponse, OpenAIError> {
    let client = Client::new();

    let system_message = ChatCompletionRequestMessage {
        role: async_openai::types::Role::System,
        content: "You are a Telegram chat bot that helps humans to understand what is happening or has happened in group chats"
            .to_string(),
        name: None,
    };

    let mut chat_history = String::new();

    for message in messages {
        let username = message.from().and_then(|user| user.username.clone());
        let message_text = message.text();
        let message_time = message.date.naive_local();

        if let (Some(username), Some(message_text)) = (username, message_text) {
            chat_history
                .push_str(format!("{} [{}]: {}\n", username, message_time, message_text).as_str())
        }
    }

    let task_message = ChatCompletionRequestMessage {
        role: async_openai::types::Role::User,
        content: format!(
            "Can you summarize the following conversation: \n\n '{}' ? ",
            chat_history
        ),
        name: None,
    };

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model("gpt-4")
        .messages(vec![system_message, task_message])
        .build()?;

    client.chat().create(request).await
}

#[instrument]
pub async fn reply(
    msgs: &[ChatCompletionRequestMessage],
    system: Option<&str>,
    model: Option<&str>,
) -> Result<CreateChatCompletionResponse, OpenAIError> {
    let client = Client::new();

    let system_msg = ChatCompletionRequestMessage {
        role: async_openai::types::Role::System,
        content: system
            .unwrap_or("You are GTP-4 a Telegram chat bot")
            .to_string(),
        name: None,
    };

    let mut req_msgs = vec![system_msg];

    req_msgs.extend_from_slice(msgs);

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model(model.unwrap_or("gpt-4"))
        .messages(msgs)
        .build()?;

    client.chat().create(request).await
}
