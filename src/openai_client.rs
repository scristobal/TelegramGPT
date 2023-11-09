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

#[instrument]
pub async fn reply(
    messages: &[ChatCompletionRequestMessage],
    client: Option<Client>,
    system: Option<&str>,
    model: Option<&str>,
) -> Result<ChatCompletionResponseStream, OpenAIError> {
    let client = client.unwrap_or_else(Client::new);

    let system = system
        .unwrap_or("You are GTP-4 a Telegram chat bot")
        .to_string();

    let system_msg = ChatCompletionRequestMessage {
        role: Role::System,
        content: system,
        name: None,
    };

    let model = model.unwrap_or("gpt-4-1106-preview");

    let mut request_messages = Vec::new();

    request_messages.push(system_msg);

    for message in messages.iter() {
        let Ok(max_tokens) = get_chat_completion_max_tokens("gpt-4", &request_messages) else { break };

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
