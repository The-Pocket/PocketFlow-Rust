use crate::state::RagState;
use anyhow::Result;
use async_trait::async_trait;
use pocketflow_rs::embedding::EmbeddingGenerator;
use pocketflow_rs::utils::embedding::{EmbeddingOptions, OpenAIEmbeddingGenerator};
use pocketflow_rs::{Context, Node, ProcessResult};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::{debug, info};

pub struct EmbedDocumentsNode {
    generator: Arc<OpenAIEmbeddingGenerator>,
}

impl EmbedDocumentsNode {
    pub fn new(api_key: String, endpoint: String, model: String, dimension: Option<usize>) -> Self {
        Self {
            generator: Arc::new(OpenAIEmbeddingGenerator::new(
                &api_key,
                &endpoint,
                EmbeddingOptions {
                    model,
                    dimensions: dimension,
                },
            )),
        }
    }
}

#[async_trait]
impl Node for EmbedDocumentsNode {
    type State = RagState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let documents_chunked = context
            .get("documents_chunked")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("No chunks found in context"))?;
        info!("Documents chunked: {:?}", documents_chunked.len());

        let mut embed_result = Vec::new();
        for chunk in documents_chunked {
            let chunks = chunk
                .get("chunks")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow::anyhow!("No chunks found in document"))?;
            let chunk_text: Vec<String> = chunks
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            debug!("Chunk text: {:?}", chunk_text);
            info!("Chunk text len: {:?}", chunk_text.len());
            let embeddings = self.generator.generate_embeddings(&chunk_text).await?;
            info!("Embeddings len: {:?}", embeddings.len());
            if embeddings.is_empty() {
                return Err(anyhow::anyhow!("Embeddings array is empty"));
            }
            info!("First Embeddings: {:?}", embeddings[0]);

            embed_result.push(json!(
                {
                    "chunks": chunk_text,
                    "embeddings": embeddings,
                    "metadata": chunk.get("metadata").unwrap_or(&Value::Null),
                }
            ));
        }

        Ok(Value::Array(embed_result))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<RagState>> {
        match result {
            Ok(value) => {
                context.set("chunk_embeddings", value.clone());
                Ok(ProcessResult::new(
                    RagState::Default,
                    "chunks_embedded".to_string(),
                ))
            }
            Err(e) => Ok(ProcessResult::new(
                RagState::EmbeddingError,
                format!("embedding_error: {}", e),
            )),
        }
    }
}
