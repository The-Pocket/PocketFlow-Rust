use anyhow::{Context, Result};
use async_trait::async_trait;
use pocketflow_rs::{Context as FlowContext, Node, ProcessResult};
use reqwest::header::HeaderValue;
use serde_json::{json, Value};
use tracing::info;
use std::path::Path;
use std::sync::Arc;
use crate::state::RagState;
use reqwest::Client;
use std::time::SystemTime;
use pdf_extract::extract_text;
use std::fs;

#[derive(Debug)]
struct Document {
    content: String,
    metadata: Value,
}

impl Document {
    fn new(content: String, url: &str, file_type: &str) -> Self {
        let metadata = json!({
            "url": url,
            "file_type": file_type,
            "timestamp": SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            "content_length": content.len(),
        });
        Self { content, metadata }
    }
}

pub struct FileLoaderNode {
    urls: Vec<String>,
    client: Arc<Client>,
}

impl FileLoaderNode {
    pub fn new(urls: Vec<String>) -> Self {
        Self {
            urls,
            client: Arc::new(Client::new()),
        }
    }

    fn detect_file_type(path: &Path) -> Result<&'static str> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine file extension"))?;

        match extension.to_lowercase().as_str() {
            "pdf" => Ok("pdf"),
            "txt" => Ok("text"),
            _ => Err(anyhow::anyhow!("Unsupported file type: {}", extension)),
        }
    }

    async fn load_from_url(&self, url: &str) -> Result<Document> {
        info!("Loading content from URL: {}", url);
        if url.starts_with("http://") || url.starts_with("https://") {
            let response = self.client.get(url).send().await?;
            let content_type = response.headers().get("content-type").map(|header| header.to_str().unwrap_or("text/plain"));
            
            let mut file_type = "web";
            let content = match content_type {
                Some("text/plain") => response.text().await?,
                Some("application/pdf") => {
                    let bytes = response.bytes().await?;
                    file_type = "pdf";
                    pdf_extract::extract_text_from_mem(&bytes)?
                }
                _ => response.text().await?,
            };

            Ok(Document::new(content, url, file_type))
        } else {
            info!("Loading content from local file: {}", url);
            let path = Path::new(url);
            let file_type = Self::detect_file_type(path)?;
            let content = match file_type {
                "pdf" => extract_text(path)
                    .with_context(|| format!("Failed to extract text from PDF: {:?}", path))?,
                "text" => fs::read_to_string(path)
                    .with_context(|| format!("Failed to read text file: {:?}", path))?,
                _ => unreachable!(),
            };
            Ok(Document::new(content, url, file_type))
        }
    }
}

#[async_trait]
impl Node for FileLoaderNode {
    type State = RagState;

    #[allow(unused_variables)]
    async fn execute(&self, context: &FlowContext) -> Result<Value> {
        let mut documents = Vec::new();
        
        for url in &self.urls {
            let doc = self.load_from_url(url).await
                .with_context(|| format!("Failed to load content from URL: {}", url))?;
            info!("Document loaded: {:?}", doc.metadata);
            documents.push(json!({
                "content": doc.content,
                "metadata": doc.metadata
            }));
        }

        if documents.is_empty() {
            return Err(anyhow::anyhow!("No documents loaded from any URL"));
        }

        Ok(Value::Array(documents))
    }

    async fn post_process(
        &self,
        context: &mut FlowContext,
        result: &Result<Value>,
    ) -> Result<ProcessResult<RagState>> {
        match result {
            Ok(value) => {
                context.set("documents", value.clone());
                Ok(ProcessResult::new(
                    RagState::Default,
                    "documents_loaded".to_string(),
                ))
            }
            Err(e) => Ok(ProcessResult::new(
                RagState::FileLoadedError,
                format!("loading_error: {}", e),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_load_text_file() {
        // Create a temporary directory
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        
        // Create a test text file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Hello, World!").unwrap();
        
        // Test loading the text file
        let loader = FileLoaderNode::new(vec![file_path.to_str().unwrap().to_string()]);
        let result = loader.execute(&FlowContext::new()).await.unwrap();
        
        // Verify the result
        let documents = result.as_array().unwrap();
        assert_eq!(documents.len(), 1);
        
        let doc = &documents[0];
        assert_eq!(doc["content"].as_str().unwrap(), "Hello, World!\n");
        assert_eq!(doc["metadata"]["file_type"].as_str().unwrap(), "text");
    }

    #[tokio::test]
    async fn test_load_multiple_files() {
        let dir = tempdir().unwrap();
        
        let text_path = dir.path().join("test.txt");
        let mut text_file = File::create(&text_path).unwrap();
        writeln!(text_file, "Text content").unwrap();
        
        
        let urls = vec![
            text_path.to_str().unwrap().to_string(),
            "https://pdfobject.com/pdf/sample.pdf".to_string(),
        ];
        
        let loader = FileLoaderNode::new(urls);
        let result = loader.execute(&FlowContext::new()).await;
        
        if let Ok(result) = result {
            let documents = result.as_array().unwrap();
            assert!(documents.len() > 0);
            
            for doc in documents {
                assert!(doc["content"].is_string());
                assert!(doc["metadata"]["url"].is_string());
                assert!(doc["metadata"]["file_type"].is_string());
                assert!(doc["metadata"]["timestamp"].is_number());
                assert!(doc["metadata"]["content_length"].is_number());
            }
        }
    }

    #[tokio::test]
    async fn test_invalid_file_type() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.xyz");
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Some content").unwrap();
        
        let loader = FileLoaderNode::new(vec![file_path.to_str().unwrap().to_string()]);
        let result = loader.execute(&FlowContext::new()).await;
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Failed to load content from URL"));
    }
}
