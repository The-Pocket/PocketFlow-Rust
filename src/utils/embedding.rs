#![cfg(feature = "openai")]

use async_trait::async_trait;
use openai_api_rust::*;
use openai_api_rust::embeddings::*;
use tracing::info;

#[derive(Debug, Clone)]
pub struct EmbeddingOptions {
    pub model: String,
    pub dimensions: Option<usize>,
}

impl Default for EmbeddingOptions {
    fn default() -> Self {
        Self {
            model: "text-embedding-ada-002".to_string(),
            dimensions: None,
        }
    }
}

#[async_trait]
pub trait EmbeddingGenerator {
    async fn generate_embedding(&self, text: &str) -> anyhow::Result<Vec<f64>>;
    async fn generate_embeddings(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f64>>>;
}

#[allow(dead_code)]
pub struct OpenAIEmbeddingGenerator {
    api_key: String,
    options: EmbeddingOptions,
    client: OpenAI,
}

impl OpenAIEmbeddingGenerator {
    pub fn new(
        api_key: &str,
        endpoint: &str,
        options: EmbeddingOptions
    ) -> Self {
        let auth = Auth::new(&api_key);
        let client = OpenAI::new(auth, endpoint);
        Self { api_key: api_key.to_string(), options: options, client: client }
    }
}

#[async_trait]
impl EmbeddingGenerator for OpenAIEmbeddingGenerator {
    async fn generate_embedding(&self, text: &str) -> anyhow::Result<Vec<f64>> {
        let embeds = self.generate_embeddings(&vec![text.to_string()]).await?;
        let result: Vec<f64> = embeds[0].iter().cloned().collect();
        Ok(result)
    }

    async fn generate_embeddings(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f64>>> {
        // chunked by 10
        let chunks = texts.chunks(10).collect::<Vec<_>>();
        let mut results = Vec::new();
        for chunk in chunks {
            let embedding = EmbeddingsBody { 
                model: self.options.model.clone(),
                input: chunk.to_vec(),
                user: None,
            };

            info!("Sending request to OpenAI Embedding API");
            let response = self.client.embeddings_create(&embedding).unwrap();
            let data = response.data.unwrap();
            let result: Vec<Vec<f64>> = data.into_iter().map(
                |x: EmbeddingData|{
                    x.embedding.unwrap()
                }
            ).collect();
            results.extend(result);
        }
        Ok(results)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::env;


    #[tokio::test]
    #[ignore = "E2E case, requires API keys"]
    async fn test_e2e_embedding_generator() {
        let generator = OpenAIEmbeddingGenerator::new(
            &env::var("DASH_SCOPE_API_KEY").unwrap(),
            "https://dashscope.aliyuncs.com/compatible-mode/v1/",
            EmbeddingOptions{
                model: "text-embedding-v3".to_string(),
                dimensions: Some(64),
            },
        );
        let text = "Hello, world!";
        let embedding = generator.generate_embedding(&text).await.unwrap();
        println!("{:?}", embedding);
    }
}