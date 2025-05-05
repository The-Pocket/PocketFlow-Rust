use anyhow::Result;
use async_trait::async_trait;
use pocketflow_rs::{Context, Node, ProcessResult};
use pocketflow_rs::utils::embedding::{OpenAIEmbeddingGenerator, EmbeddingOptions, EmbeddingGenerator};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::state::RagState;

pub struct EmbedQueryNode {
    query: String,
    generator: Arc<OpenAIEmbeddingGenerator>,
}

impl EmbedQueryNode {
    pub fn new(query: String, api_key: String, endpoint: String) -> Self {
        Self {
            query,
            generator: Arc::new(OpenAIEmbeddingGenerator::new(
                &api_key,
                &endpoint,
                EmbeddingOptions::default(),
            )),
        }
    }
}

#[async_trait]
impl Node for EmbedQueryNode {
    type State = RagState;

    #[allow(unused_variables)]
    async fn execute(&self, context: &Context) -> Result<Value> {
        let embedding = self.generator.generate_embedding(&self.query).await?;
        if embedding.is_empty() {
            return Err(anyhow::anyhow!("No embedding generated for query"));
        }
        Ok(Value::Array(
            embedding
                .into_iter()
                .map(|x| json!(x))
                .collect(),
        ))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<RagState>> {
        match result {
            Ok(value) => {
                context.set("query_embedding", value.clone());
                Ok(ProcessResult::new(
                    RagState::Default,
                    "query_embedded".to_string(),
                ))
            }
            Err(e) => Ok(ProcessResult::new(
                RagState::QueryEmbeddingError,
                format!("query_embedding_error: {}", e),
            )),
        }
    }
} 