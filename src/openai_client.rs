use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionResponseStream, CreateChatCompletionRequestArgs,
};
use async_openai::{Client, config::OpenAIConfig, error::OpenAIError};
use tracing::instrument;

const MAX_TOKENS_COMPLETION: u16 = 1_000;
const DEFAULT_MODEL: &str = "gpt-4o-2024-11-20";
const DEFAULT_SYSTEM_MESSAGE: &str = "You are a helpful Telegram chat bot";

#[instrument]
pub async fn reply(
    message_history: &[ChatCompletionRequestMessage],
    client: Option<Client<OpenAIConfig>>,
    system_message: Option<&str>,
    model: Option<&str>,
) -> Result<ChatCompletionResponseStream, OpenAIError> {
    let client = client.unwrap_or_else(Client::new);

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(MAX_TOKENS_COMPLETION)
        .model(model.unwrap_or(DEFAULT_MODEL))
        .messages(message_history.to_vec())
        .build()?;

    client.chat().create_stream(request).await
}
