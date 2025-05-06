use anyhow::Result;
use async_trait::async_trait;
use pocketflow_rs::{Context, Node, ProcessResult};
use pocketflow_rs::utils::llm_wrapper::{OpenAIClient, LLMWrapper};
use serde_json::Value;
use tracing::info;
use std::sync::Arc;
use crate::state::RagState;

pub struct QueryRewriteNode {
    client: Arc<OpenAIClient>,
}

impl QueryRewriteNode {
    pub fn new(api_key: String, model: String, endpoint: String) -> Self {
        Self {
            client: Arc::new(OpenAIClient::new(api_key, model, endpoint)),
        }
    }
}

#[async_trait]
impl Node for QueryRewriteNode {
    type State = RagState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let user_query = context.get("user_query").unwrap();
        let prompt = format!("
**Role:** You are an AI Query Enhancer for a Retrieval-Augmented Generation (RAG) system.

**Goal:** Your task is to take a raw user query and rewrite it into an optimized query string suitable for vector database search. This involves identifying the user's core intent and transforming the query into a concise, keyword-focused format that maximizes the chances of retrieving relevant documents.

**Input:** You will receive a single \"Original User Query\".

**Instructions:**

1.  **Analyze Intent:** Carefully examine the \"Original User Query\" to understand the user's underlying information need or question. What are they *really* trying to find out?
2.  **Identify Keywords:** Extract the most critical entities, concepts, and keywords from the query.
3.  **Remove Filler:** Discard conversational filler, politeness phrases (e.g., \"please\", \"can you tell me\"), and vague phrasing (\"thing\", \"stuff\", \"how about\").
4.  **Rewrite for Clarity & Conciseness:** Construct a new query string that clearly represents the intent using the identified keywords. Make it specific and direct.
5.  **Consider Expansion (Optional but Recommended):** If the original query is very sparse or could benefit from clarification, cautiously add 1-2 highly relevant synonyms or closely related terms that specify the intent further (e.g., adding \"nutrition\" if the query is just \"apples\"). Avoid overly broad expansion.
6.  **Format for Embedding:** The final rewritten query should be a simple string, optimized for being turned into a vector embedding for semantic search.

**Output:** Respond with ONLY the rewritten query string. Do not include any explanations or introductory text.

**Example 1:**
Original User Query: \"Hey, could you tell me about the financial performance of Tesla last year?\"
Rewritten Query: `Tesla financial performance 2024 earnings report revenue analysis`

**Example 2:**
Original User Query: \"What's the deal with that new AI that makes pictures?\"
Rewritten Query: `AI image generation model technology explanation diffusion transformer`

**Example 3:**
Original User Query: \"I need help understanding how to mitigate risks in my supply chain in Europe.\"
Rewritten Query: `supply chain risk mitigation strategies Europe logistics management`

**Now, process the following input:**

Original User Query: \"{}\"
Rewritten Query:",user_query);
        let response = self.client.generate(&prompt).await?;
        info!("Query rewritten: {:?}", response.content);
        Ok(Value::String(response.content.replace("`", "")))
    }


    #[allow(unused_variables)]
    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<RagState>> {
        return match result {
            Ok(value) => {
                context.set("rewritten_query", value.clone());
                Ok(ProcessResult::new(RagState::Default, "query_rewritten".to_string()))
            }
            Err(e) => {
                info!("Error rewriting query: {:?}", e);
                Ok(ProcessResult::new(RagState::QueryRewriteError, "query_rewrite_error".to_string()))
            }
        }
    }

}
