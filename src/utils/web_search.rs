use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[async_trait]
pub trait WebSearcher {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<SearchResult>>;
    async fn search_with_options(&self, query: &str, options: SearchOptions) -> anyhow::Result<Vec<SearchResult>>;
}

#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub max_results: Option<usize>,
    pub language: Option<String>,
    pub region: Option<String>,
}

pub struct GoogleSearcher {
    api_key: String,
    search_engine_id: String,
    client: Client,
}

impl GoogleSearcher {
    pub fn new(api_key: String, search_engine_id: String) -> Self {
        Self {
            api_key,
            search_engine_id,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl WebSearcher for GoogleSearcher {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<SearchResult>> {
        self.search_with_options(query, SearchOptions::default()).await
    }

    async fn search_with_options(&self, query: &str, options: SearchOptions) -> anyhow::Result<Vec<SearchResult>> {
        let mut url = format!(
            "https://www.googleapis.com/customsearch/v1?key={}&cx={}&q={}",
            self.api_key, self.search_engine_id, query
        );

        if let Some(lang) = options.language {
            url.push_str(&format!("&lr=lang_{}", lang));
        }
        if let Some(region) = options.region {
            url.push_str(&format!("&cr=country{}", region));
        }
        if let Some(max_results) = options.max_results {
            url.push_str(&format!("&num={}", max_results));
        }

        info!("Sending request to Google Search API");
        let response = self.client.get(&url).send().await?;
        let search_response: serde_json::Value = response.json().await?;
        let default_val: Vec<serde_json::Value> = vec![];
        let items = search_response["items"].as_array().unwrap_or(&default_val);
        let results = items
            .iter()
            .map(|item| SearchResult {
                title: item["title"].as_str().unwrap_or("").to_string(),
                url: item["link"].as_str().unwrap_or("").to_string(),
                snippet: item["snippet"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_e2e_google_searcher() {
        let searcher = GoogleSearcher::new(
            env::var("GOOGLE_API_KEY").unwrap(),
            env::var("GOOGLE_SEARCH_ENGINE_ID").unwrap(),
        );
        let results = searcher.search("Beijing's temperature today").await.unwrap();
        println!("{:?}", results);
    }
}