use anyhow::Result;
use async_trait::async_trait;
use pocketflow_rs::{Context, Node, ProcessResult};
use pocketflow_rs::utils::text_chunking::{TextChunker, ChunkingOptions, ChunkingStrategy};
use serde_json::{json, Value};
use tracing::info;
use crate::state::RagState;

pub struct ChunkDocumentsNode {
    chunker: TextChunker,
    options: ChunkingOptions,
}

impl ChunkDocumentsNode {
    pub fn new(chunk_size: usize, overlap: usize, strategy: ChunkingStrategy) -> Self {
        Self {
            chunker: TextChunker::new(),
            options: ChunkingOptions {
                chunk_size,
                overlap,
                strategy,
            },
        }
    }
}

#[async_trait]
impl Node for ChunkDocumentsNode {
    type State = RagState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let documents = context
            .get("documents")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("No documents found in context"))?;
        
        let mut chunks_meta = Vec::new();
        for doc_map in documents {
            let content = doc_map.get("content").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("No content found in document"))?;
            let chunks = self.chunker.chunk_text(content, &self.options);
            info!("Process: {:?}, Chunks lens: {:?}", doc_map.get("metadata").unwrap(), chunks.len());
            chunks_meta.push(json!({
                "chunks": chunks,
                "metadata": doc_map.get("metadata").unwrap_or(&Value::Null),
            }));
        }

        Ok(Value::Array(chunks_meta))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<RagState>> {
        match result {
            Ok(value) => {
                context.set("documents_chunked", value.clone());
                Ok(ProcessResult::new(
                    RagState::Default,
                    "documents_chunked".to_string(),
                ))
            }
            Err(e) => Ok(ProcessResult::new(
                RagState::ChunkingError,
                format!("chunking_error: {}", e),
            )),
        }
    }
} 