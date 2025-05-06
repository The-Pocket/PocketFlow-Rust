use anyhow::Result;
use async_trait::async_trait;
use pocketflow_rs::vector_db::{DistanceMetric, VectorDBOptions};
use pocketflow_rs::{Context, Node, ProcessResult};
use pocketflow_rs::utils::vector_db::{QdrantDB, VectorDB};
use serde_json::Value;
use tracing::{info, error};
use std::sync::Arc;
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

        let records= self.db.search(query_embedding, self.k).await?;
        if records.is_empty() {
            error!("No documents retrieved");
            return Err(anyhow::anyhow!("No documents retrieved"));
        }

        info!("Retrieved documents line: {:?}", records.len());
        
        let result_array: Vec<Value> = records.into_iter().map(|record| record.to_value()).collect();
        
        Ok(Value::Array(result_array))
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