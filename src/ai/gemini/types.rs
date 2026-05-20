use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Clone)]
pub struct EmbedRequest {
    pub model: String,
    pub content: EmbedContentPart,
}

#[derive(Serialize, Clone)]
pub struct EmbedContentPart {
    pub parts: Vec<TextPart>,
}

#[derive(Serialize, Clone)]
pub struct GeminiRequest {
    pub contents: Vec<ContentsPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "generationConfig")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Serialize, Clone)]
pub struct ContentsPart {
    pub parts: Vec<TextPart>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TextPart {
    pub text: String,
}

#[derive(Serialize, Clone)]
pub struct GenerationConfig {
    #[serde(rename = "responseMimeType")]
    pub response_mime_type: String,
    #[serde(rename = "responseSchema")]
    pub response_schema: Value,
}

#[derive(Deserialize)]
pub struct EmbedResponse {
    pub embedding: EmbeddingData,
}

#[derive(Deserialize)]
pub struct EmbeddingData {
    pub values: Vec<f32>,
}

/// Response from GET /v1beta/models
#[derive(Deserialize, Debug)]
pub struct ModelListResponse {
    pub models: Vec<ModelInfo>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    #[serde(rename = "displayName")]
    #[allow(dead_code)]
    pub display_name: String,
    #[serde(rename = "supportedGenerationMethods", default)]
    pub supported_methods: Vec<String>,
    #[serde(rename = "inputTokenLimit", default)]
    pub input_token_limit: u32,
    #[serde(rename = "outputTokenLimit", default)]
    #[allow(dead_code)]
    pub output_token_limit: u32,
    #[serde(rename = "outputDimensionality", default)]
    pub output_dimensionality: Option<u32>,
    #[serde(default)]
    #[allow(dead_code)]
    pub description: String,
}

impl ModelInfo {
    pub fn supports_embed(&self) -> bool {
        self.supported_methods.iter().any(|m| m == "embedContent")
    }

    pub fn supports_generate(&self) -> bool {
        self.supported_methods.iter().any(|m| m == "generateContent")
    }
}
