use reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use tracing::{info, instrument};

const MODEL_URL: &str = "https://api.replicate.com/v1/predictions";

#[derive(Deserialize, Debug, Clone)]
pub struct Response<Input, Output> {
    completed_at: Option<String>,
    created_at: Option<String>,
    error: Option<String>,
    hardware: Option<String>,
    id: String,
    input: Input,
    logs: String,
    metrics: Metrics,
    output: Output,
    started_at: Option<String>,
    status: String,
    urls: Urls,
    version: String,
    webhook_completed: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct Metrics {
    predict_time: f32,
}

#[derive(Deserialize, Debug, Clone)]
struct Urls {
    get: String,
    cancel: String,
}

#[skip_serializing_none]
#[derive(Serialize, Debug)]
struct Request<I> {
    version: String,
    input: I,
    webhook_completed: Option<String>,
}

const MODEL_VERSION: &str = "328bd9692d29d6781034e3acab8cf3fcb122161e6f5afb896a4ca9fd57090577";

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Input {
    prompt: String,
    seed: Option<u32>,
    num_inference_steps: Option<u32>,
    guidance_scale: Option<f32>,
}

type Output = Option<Vec<String>>;

async fn api_call<Request: Serialize>(
    request: &Request,
) -> Result<reqwest::Response, reqwest::Error> {
    let client = Client::new();

    let token = std::env::var("REPLICATE_TOKEN").expect("REPLICATE_TOKEN must be set");

    let body = serde_json::to_string(&request).unwrap();

    client
        .post(MODEL_URL.to_string())
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, "Token ".to_string() + &token)
        .body(body)
        .send()
        .await
}

#[instrument]
pub async fn draw(prompt: String) -> Response<Input, Output> {
    let request = Request {
        version: MODEL_VERSION.to_string(),
        input: Input {
            prompt,
            seed: None,
            num_inference_steps: None,
            guidance_scale: None,
        },
        webhook_completed: None,
    };

    info!(?request);

    let api_response = api_call(&request).await.unwrap();

    info!(?api_response);

    let response = api_response
        .json::<Response<Input, Output>>()
        .await
        .unwrap();

    info!(?response);

    response
}
