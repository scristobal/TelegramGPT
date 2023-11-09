use async_openai::{
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, ChatCompletionResponseStream,
        CreateChatCompletionRequestArgs, Role,
    },
    Client,
};
use tiktoken_rs::get_chat_completion_max_tokens;
use tracing::instrument;

const MAX_TOKENS_COMPLETION: u16 = 1_000;
const DEFAULT_MODEL: &str = "gpt-4-1106-preview";
const DEFAULT_SYSTEM_MESSAGE: &str = "You are a helpful Telegram chat bot";

#[instrument]
pub async fn reply(
    message_history: &[ChatCompletionRequestMessage],
    client: Option<Client>,
    system_message: Option<&str>,
    model: Option<&str>,
) -> Result<ChatCompletionResponseStream, OpenAIError> {
    let client = client.unwrap_or_else(Client::new);

    let system = system_message.unwrap_or(DEFAULT_SYSTEM_MESSAGE).to_string();

    let system_msg = ChatCompletionRequestMessage {
        role: Role::System,
        content: system,
        name: None,
    };

    let model = model.unwrap_or(DEFAULT_MODEL);

    let mut request_messages = Vec::new();

    request_messages.push(system_msg);

    for message in message_history.iter() {
        let Ok(max_tokens) = get_chat_completion_max_tokens(DEFAULT_MODEL, &request_messages) else { break };

        if max_tokens < (MAX_TOKENS_COMPLETION as usize) {
            break;
        };

        request_messages.push(message.clone());
    }

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(MAX_TOKENS_COMPLETION)
        .model(model)
        .messages(request_messages)
        .build()?;

    client.chat().create_stream(request).await
}
