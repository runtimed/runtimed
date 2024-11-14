use anyhow::Context as _;
use futures::{Stream, StreamExt as _};
use reqwest::Client;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Assistant,
    User,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Serialize)]
pub struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    // keep_alive: Option<Duration>,
}

#[derive(Deserialize)]
pub struct ChatResponse {
    #[allow(unused)]
    pub model: String,
    #[allow(unused)]
    pub created_at: String,
    pub message: ChatMessage,
    #[allow(unused)]
    pub done_reason: Option<String>,
    #[allow(unused)]
    pub done: bool,
}

#[derive(Serialize, Deserialize)]
pub struct LocalModelsResponse {
    pub models: Vec<LocalModelListing>,
}

#[derive(Serialize, Deserialize)]
pub struct LocalModelListing {
    pub name: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
    pub details: ModelDetails,
}

#[derive(Serialize, Deserialize)]
pub struct LocalModel {
    pub modelfile: String,
    pub parameters: Option<String>,
    pub template: String,
    pub details: ModelDetails,
}

#[derive(Serialize, Deserialize)]
pub struct ModelDetails {
    pub format: String,
    pub family: String,
    pub families: Option<Vec<String>>,
    pub parameter_size: String,
    pub quantization_level: String,
}

#[derive(Serialize, Deserialize)]
pub struct ModelCreateResponse {
    pub status: String,
}

pub const OLLAMA_ENDPOINT: &str = "http://localhost:11434";

pub struct OllamaClient {
    client: Client,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Json,
}

#[derive(Serialize, Deserialize, Default)]
pub struct GenerateRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    format: Option<Format>,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    system: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    context: Vec<usize>,
}

#[derive(Serialize, Debug)]
// todo!(): Figure out of serde can tag based on a bool (`done`)
// For now we'll put these in an order that preferences the finished generation to be tried first.
pub enum GenerateResponse {
    Finished(FinishedGeneration),
    Delta(DeltaGeneration),
}

impl<'de> Deserialize<'de> for GenerateResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Value::deserialize(deserializer)?;
        if let Some(done) = v.get("done") {
            let done = done.as_bool().unwrap_or(false);

            if done {
                Ok(GenerateResponse::Finished(
                    serde_json::from_value(v)
                        .map_err(|e| D::Error::custom(format!("Invalid response format: {}", e)))?,
                ))
            } else {
                Ok(GenerateResponse::Delta(serde_json::from_value(v).map_err(
                    |e| D::Error::custom(format!("Invalid response format: {}", e)),
                )?))
            }
        } else {
            Err(D::Error::missing_field("done"))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeltaGeneration {
    pub model: String,
    pub created_at: String,
    pub response: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FinishedGeneration {
    pub model: String,
    pub created_at: String,
    pub done_reason: String,
    pub context: Vec<usize>,
}

impl OllamaClient {
    pub fn new() -> Self {
        let client = Client::new();

        Self { client }
    }

    pub async fn generate(
        &mut self,
        model: &str,
        prompt: &str,
        context: &Vec<usize>,
        format: Option<Format>,
        system: Option<&str>,
    ) -> anyhow::Result<impl Stream<Item = Result<GenerateResponse, anyhow::Error>>> {
        let request = GenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            format,
            system: system.unwrap_or_default().to_string(),
            context: context.to_owned(),
        };

        let response = self
            .client
            .post(format!("{OLLAMA_ENDPOINT}/api/generate"))
            .json(&request)
            .send()
            .await
            .context("Failed to request generate API")?;

        let stream = response.bytes_stream();
        Ok(stream.map(|res| match res {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(err) => Err(anyhow::Error::new(err)),
        }))
    }

    pub async fn chat(
        &mut self,
        model: &str,
        messages: &[ChatMessage],
    ) -> anyhow::Result<impl Stream<Item = Result<ChatResponse, anyhow::Error>>> {
        let chat_request = ChatRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            stream: true,
        };

        let response = self
            .client
            .post(format!("{OLLAMA_ENDPOINT}/api/chat"))
            .json(&chat_request)
            .send()
            .await
            .context("Failed to request chat API")?;

        let stream = response.bytes_stream();
        Ok(stream.map(|res| match res {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(err) => Err(anyhow::Error::new(err)),
        }))
    }

    pub async fn list_local_models(&mut self) -> anyhow::Result<Vec<LocalModelListing>> {
        let response = self
            .client
            .get(format!("{OLLAMA_ENDPOINT}/api/tags"))
            .send()
            .await
            .context("Failed to list local models");

        let response_text = response?.text().await?;
        let local_models: LocalModelsResponse = serde_json::from_str(&response_text)?;
        anyhow::Ok(local_models.models)
    }

    pub async fn show(&mut self, model: &str) -> anyhow::Result<LocalModel> {
        let response = self
            .client
            .post(format!("{OLLAMA_ENDPOINT}/api/show"))
            .json(&json!({
                "name": model
            }))
            .send()
            .await;

        let response_text = response?.text().await?;
        let local_model: LocalModel = serde_json::from_str(&response_text)?;
        anyhow::Ok(local_model)
    }

    pub async fn create(
        &mut self,
        name: &str,
        modelfile_contents: &str,
    ) -> anyhow::Result<impl Stream<Item = Result<ModelCreateResponse, anyhow::Error>>> {
        let response = self
            .client
            .post(format!("{OLLAMA_ENDPOINT}/api/create"))
            .json(&json!({
                "name": name,
                "modelfile": modelfile_contents,
                "stream": true
            }))
            .send()
            .await
            .context("Failed to create model")?;

        let stream = response.bytes_stream();
        Ok(stream.map(|res| match res {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(err) => Err(anyhow::Error::new(err)),
        }))
    }
}
