use reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use tracing::info;

const MODEL_URL: &str = "https://api.replicate.com/v1/predictions";

#[derive(Deserialize, Debug, Clone)]
pub struct ReplicateResponse<ModelInput, ModelOutput> {
    completed_at: Option<String>,
    created_at: Option<String>,
    error: Option<String>,
    hardware: Option<String>,
    id: String,
    input: ModelInput,
    logs: Option<String>,
    metrics: Option<Metrics>,
    output: Option<ModelOutput>,
    started_at: Option<String>,
    status: String,
    urls: Urls,
    version: String,
    webhook_completed: Option<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Debug, Clone)]
struct Metrics {
    predict_time: Option<f32>,
}

#[derive(Deserialize, Debug, Clone)]
struct Urls {
    get: String,
    cancel: String,
}

#[skip_serializing_none]
#[derive(Serialize, Debug)]
struct ReplicateRequest<I> {
    version: String,
    input: I,
}

const MODEL_VERSION: &str = "328bd9692d29d6781034e3acab8cf3fcb122161e6f5afb896a4ca9fd57090577";

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StableDiffusionInput {
    prompt: String,
    seed: Option<u32>,
    num_inference_steps: Option<u32>,
    guidance_scale: Option<f32>,
}

type StableDiffusionOutput = Option<Vec<String>>;

type StableDiffusionRequest = ReplicateRequest<StableDiffusionInput>;
type StableDiffusionResponse = ReplicateResponse<StableDiffusionInput, StableDiffusionOutput>;

async fn api_call(
    request: &StableDiffusionRequest,
) -> Result<StableDiffusionResponse, reqwest::Error> {
    let client = Client::new();

    let token = std::env::var("REPLICATE_TOKEN").expect("REPLICATE_TOKEN must be set");

    let response = client
        .post(MODEL_URL.to_string())
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, "Token ".to_string() + &token)
        .json(request)
        .send()
        .await?;

    info!(?response);

    let response_body = response.json().await;

    info!(?response_body);
    response_body
}

pub async fn draw(prompt: String) -> Result<StableDiffusionResponse, anyhow::Error> {
    let input = StableDiffusionInput {
        prompt,
        seed: None,
        num_inference_steps: None,
        guidance_scale: None,
    };

    info!(?input);

    let request = StableDiffusionRequest {
        version: MODEL_VERSION.to_string(),
        input,
    };

    info!(?request);

    let response = api_call(&request).await?;

    info!(?response);

    Ok(response)
}
