use anyhow::Result;
use async_trait::async_trait;
use pocketflow_rs::{Context, Node, ProcessResult};
use pocketflow_rs::utils::llm_wrapper::{OpenAIClient, LLMWrapper};
use serde_json::Value;
use std::sync::Arc;
use crate::state::RagState;

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

        let context_text = retrieved_docs
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        if context_text.is_empty() {
            return Ok(Value::String("I don't know.".to_string()));
        }

        let prompt = format!(
            "You are a helpful assistant. Based on the following context, please answer the question. If the answer cannot be found in the context, say 'I don't know'.\n\nContext:\n{}\n\nQuestion: {}\n\nAnswer:",
            context_text,
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
                context.set("answer", value.clone());
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