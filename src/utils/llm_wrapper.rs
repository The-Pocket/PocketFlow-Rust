use std::{collections::HashMap, hash::RandomState};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use openai_api_rust::*;
use openai_api_rust::chat::*;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub usage: Option<LLMUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[async_trait]
pub trait LLMWrapper {
    async fn generate(&self, prompt: &str) -> anyhow::Result<LLMResponse>;
    async fn generate_with_options(&self, prompt: &str, options: LLMOptions) -> anyhow::Result<LLMResponse>;
}

#[derive(Debug, Clone, Default)]
pub struct LLMOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub logit_bias: Option<HashMap<String, String, RandomState>>
}

#[allow(dead_code)]
pub struct OpenAIClient {
    api_key: String,
    model: String,
    endpoint: String,
    client: OpenAI,
    
}

impl OpenAIClient {
    pub fn new(api_key: String, model: String, endpoint: String) -> Self {
        let auth = Auth::new(&api_key);
        let client = OpenAI::new(auth, &endpoint);
        Self { api_key, model, endpoint, client }
    }
}

#[async_trait]
impl LLMWrapper for OpenAIClient {
    async fn generate(&self, prompt: &str) -> anyhow::Result<LLMResponse> {
        self.generate_with_options(prompt, LLMOptions::default()).await
    }

    async fn generate_with_options(&self, prompt: &str, options: LLMOptions) -> anyhow::Result<LLMResponse> {
        let chat = ChatBody{
            model: self.model.clone(),
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            presence_penalty: options.presence_penalty,
            frequency_penalty: options.frequency_penalty,
            logit_bias: options.logit_bias,
            top_p: options.top_p,
            stream: Some(false),
            stop: options.stop,
            user: None,
            n: Some(1),
            messages: vec![Message {
                role: Role::User,
                content: prompt.to_string(),
            }]

        };

        info!("Sending request to OpenAI API");
        let response = self.client.chat_completion_create(&chat).unwrap();
        let choice = response.choices;
        let content = &choice[0].message.as_ref().unwrap().content;
        let u = response.usage;
        let usage = LLMUsage{
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        };

        Ok(LLMResponse { 
            content: content.clone(), 
            usage: Some(usage),
        })
    }
}