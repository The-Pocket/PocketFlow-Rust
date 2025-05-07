use crate::state::RagState;
use anyhow::Result;
use async_trait::async_trait;
use pocketflow_rs::utils::vector_db::{
    DistanceMetric, QdrantDB, VectorDB, VectorDBOptions, VectorRecord,
};
use pocketflow_rs::{Context, Node, ProcessResult};
use serde_json::Value;
use std::sync::Arc;

pub struct CreateIndexNode {
    db: Arc<QdrantDB>,
}

impl CreateIndexNode {
    pub async fn new(
        db_url: String,
        api_key: Option<String>,
        collection: String,
        dimension: usize,
        distance_metric: DistanceMetric,
    ) -> Result<Self> {
        let options = VectorDBOptions {
            collection_name: collection,
            dimension,
            distance_metric,
        };
        let db = QdrantDB::new(db_url, api_key, options).await?;
        Ok(Self { db: Arc::new(db) })
    }
}

#[async_trait]
impl Node for CreateIndexNode {
    type State = RagState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let chunks_embeddings = context
            .get("chunk_embeddings")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("No embeddings found in context"))?;

        let mut records = Vec::new();
        for chunk_embedding in chunks_embeddings {
            let chunks = chunk_embedding
                .get("chunks")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow::anyhow!("No chunks found in document"))?;
            let embeddings = chunk_embedding
                .get("embeddings")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow::anyhow!("No embeddings found in document"))?;
            let metadata = chunk_embedding.get("metadata").unwrap_or(&Value::Null);

            let chunks_size = chunks.len();
            for i in 0..chunks_size {
                let chunk = chunks[i].to_string();
                let default_embedding = Vec::new();
                let embedding = embeddings[i].as_array().unwrap_or(&default_embedding);
                let embedding_vec: Vec<f32> = embedding
                    .iter()
                    .filter_map(|v| v.as_f64().map(|x| x as f32))
                    .collect();
                records.push(VectorRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    vector: embedding_vec,
                    metadata: serde_json::Map::from_iter(vec![
                        ("text".to_string(), serde_json::Value::String(chunk)),
                        ("file_metadata".to_string(), metadata.clone()),
                    ]),
                });
            }
        }

        if records.is_empty() {
            return Err(anyhow::anyhow!("No valid records to insert"));
        }

        self.db
            .insert(records)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to insert records: {}", e))?;
        Ok(Value::Null)
    }

    #[allow(unused_variables)]
    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<RagState>> {
        match result {
            Ok(_) => Ok(ProcessResult::new(
                RagState::Default,
                "index_created".to_string(),
            )),
            Err(e) => Ok(ProcessResult::new(
                RagState::IndexCreationError,
                format!("index_creation_error: {}", e),
            )),
        }
    }
}
