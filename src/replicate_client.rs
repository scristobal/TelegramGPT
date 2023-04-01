use reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Client, Url,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::{str::FromStr, time::Duration};
use tokio::time::sleep;

const MODEL_URL: &str = "https://api.replicate.com/v1/predictions";

#[derive(Deserialize, Debug, Clone)]
pub struct ReplicateResponse<ModelInput, ModelOutput> {
    completed_at: Option<String>,
    created_at: Option<String>,
    pub error: Option<String>,
    hardware: Option<String>,
    id: String,
    input: ModelInput,
    logs: Option<String>,
    metrics: Option<Metrics>,
    pub output: Option<ModelOutput>,
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
    num_outputs: Option<u32>,
}

type StableDiffusionOutput = Option<Vec<String>>;

type StableDiffusionRequest = ReplicateRequest<StableDiffusionInput>;
type StableDiffusionResponse = ReplicateResponse<StableDiffusionInput, StableDiffusionOutput>;

#[derive(Debug, Clone)]
pub struct ReplicateClient {
    http_client: Client,
    token: String,
    model_url: Url,
    model_version: &'static str,
}

impl ReplicateClient {
    pub fn new() -> Self {
        let token = std::env::var("REPLICATE_TOKEN").expect("REPLICATE_TOKEN must be set");

        let model_url = Url::from_str(MODEL_URL).unwrap();

        let mode_version = MODEL_VERSION;

        Self {
            http_client: Client::new(),
            token,
            model_url,
            model_version: mode_version,
        }
    }

    pub async fn image(&self, prompt: String) -> Result<StableDiffusionResponse, anyhow::Error> {
        let input = StableDiffusionInput {
            prompt,
            seed: None,
            num_inference_steps: None,
            guidance_scale: None,
            num_outputs: Some(4),
        };

        let request = StableDiffusionRequest {
            version: self.model_version.to_string(),
            input,
        };

        let response = self.model_request(&request).await?;

        let job_url = Url::from_str(&response.urls.get)?;

        let mut response = self.model_response(job_url.clone()).await?;

        while response.output.is_none() {
            sleep(Duration::from_millis(1000)).await;
            response = self.model_response(job_url.clone()).await?;
        }

        Ok(response)
    }

    async fn model_request(
        &self,
        request: &StableDiffusionRequest,
    ) -> Result<StableDiffusionResponse, reqwest::Error> {
        let response = self
            .http_client
            .post(self.model_url.clone())
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, "Token ".to_string() + &self.token)
            .json(request)
            .send()
            .await?;

        response.json().await
    }

    async fn model_response(&self, url: Url) -> Result<StableDiffusionResponse, reqwest::Error> {
        let response = self
            .http_client
            .get(url)
            .header(AUTHORIZATION, "Token ".to_string() + &self.token)
            .send()
            .await?;

        response.json().await
    }
}
