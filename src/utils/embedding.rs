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
    async fn generate_embedding(&self, text: &String) -> anyhow::Result<Vec<f64>>;
    async fn generate_embeddings(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f64>>>;
}

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
    async fn generate_embedding(&self, text: &String) -> anyhow::Result<Vec<f64>> {
        let embeds = self.generate_embeddings(&vec![text.clone()]).await?;
        let result: Vec<f64> = embeds[0].iter().cloned().collect();
        Ok(result)
    }

    async fn generate_embeddings(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f64>>> {
        let embedding = EmbeddingsBody { 
            model: self.options.model.clone(),
            input: texts.to_vec(),
            user: None,
        };

        info!("Sending request to OpenAI Embedding API");
        let response = self.client.embeddings_create(&embedding).unwrap();
        let data = response.data.unwrap();
        let result = data.into_iter().map(
            |x: EmbeddingData|{
                x.embedding.unwrap()
            }
        ).collect();
    
        
        Ok(result)
    }
}