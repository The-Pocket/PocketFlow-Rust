use anyhow::Result;
use async_trait::async_trait;
use pocketflow_rs::vector_db::{DistanceMetric, VectorDBOptions};
use pocketflow_rs::{Context, Node, ProcessResult};
use pocketflow_rs::utils::vector_db::{QdrantDB, VectorDB};
use serde_json::Value;
use std::sync::Arc;
use qdrant_client::Qdrant;
use crate::state::RagState;

pub struct RetrieveDocumentNode {
    db: Arc<QdrantDB>,
    k: usize,
}

impl RetrieveDocumentNode {
    pub async fn new(db_url: String, api_key: Option<String>, collection: String, dimension: usize, distance_metric: DistanceMetric, k: usize) -> Result<Self> {
        let db = QdrantDB::new(
            db_url,
            api_key,
            VectorDBOptions { collection_name: collection, dimension, distance_metric },
        ).await?;
        Ok(Self { db: Arc::new(db), k })
    }
}

#[async_trait]
impl Node for RetrieveDocumentNode {
    type State = RagState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let query_embedding = context
            .get("query_embedding")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_f64().map(|x| x as f32))
                    .collect::<Vec<f32>>()
            })
            .ok_or_else(|| anyhow::anyhow!("No query embedding found in context"))?;

        let results = self.db.search(query_embedding, self.k).await?;
        if results.is_empty() {
            return Err(anyhow::anyhow!("No documents retrieved"));
        }

        let texts: Vec<String> = results
            .into_iter()
            .filter_map(|record| {
                record
                    .metadata
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        if texts.is_empty() {
            return Err(anyhow::anyhow!("No valid text content in retrieved documents"));
        }

        Ok(Value::Array(texts.into_iter().map(Value::String).collect()))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<RagState>> {
        match result {
            Ok(value) => {
                context.set("retrieved_documents", value.clone());
                Ok(ProcessResult::new(
                    RagState::Default,
                    "documents_retrieved".to_string(),
                ))
            }
            Err(e) => Ok(ProcessResult::new(
                RagState::RetrievalError,
                format!("retrieval_error: {}", e),
            )),
        }
    }
} 