use crate::state::RagState;
use anyhow::Result;
use async_trait::async_trait;
use pocketflow_rs::utils::llm_wrapper::{LLMWrapper, OpenAIClient};
use pocketflow_rs::vector_db::VectorRecord;
use pocketflow_rs::{Context, Node, ProcessResult};
use serde_json::Value;
use std::sync::Arc;

pub struct GenerateAnswerNode {
    client: Arc<OpenAIClient>,
    query: String,
}

impl GenerateAnswerNode {
    pub fn new(api_key: String, model: String, endpoint: String, query: String) -> Self {
        Self {
            client: Arc::new(OpenAIClient::new(api_key, model, endpoint)),
            query,
        }
    }
}

#[async_trait]
impl Node for GenerateAnswerNode {
    type State = RagState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let retrieved_docs = context
            .get("retrieved_documents")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("No retrieved documents found in context"))?;

        let retrieved_docs_array: Vec<VectorRecord> = retrieved_docs
            .iter()
            .map(VectorRecord::parse_by_value)
            .collect();

        let retrieved_text_with_meta = retrieved_docs_array
            .iter()
            .map(|v| {
                format!(
                    "{}: {}",
                    v.metadata
                        .get("file_metadata")
                        .unwrap()
                        .get("url")
                        .unwrap()
                        .as_str()
                        .unwrap(),
                    v.metadata.get("text").unwrap()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        if retrieved_text_with_meta.is_empty() {
            return Ok(Value::String("I don't know.".to_string()));
        }

        let prompt = format!("
You are a helpful assistant. Based on the following context, please answer the question. If the answer cannot be found in the context, say 'I don't know'.\n\n
Output format using markdown and add reference links to the source documents. \n\n
You can use the following context to answer the question: \n{}\n\n
Question: {}\n\n
Answer:",
        retrieved_text_with_meta,
            self.query
        );

        let response = self.client.generate(&prompt).await?;
        if response.content.is_empty() {
            return Err(anyhow::anyhow!("Empty response from LLM"));
        }

        Ok(Value::String(response.content.trim().to_string()))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<RagState>> {
        match result {
            Ok(value) => {
                context.set("result", value.clone());
                Ok(ProcessResult::new(
                    RagState::Default,
                    "answer_generated".to_string(),
                ))
            }
            Err(e) => Ok(ProcessResult::new(
                RagState::GenerationError,
                format!("generation_error: {}", e),
            )),
        }
    }
}
