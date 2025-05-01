use crate::{context::Context, Params};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait Node: Send + Sync {
    #[allow(unused_variables)]
    async fn prepare(&self, context: &mut Context) -> Result<()> {
        Ok(())
    }

    async fn execute(&self, context: &Context) -> Result<serde_json::Value>;

    #[allow(unused_variables)]
    async fn post_process(&self, context: &mut Context, result: &serde_json::Value) -> Result<&str> {
        Ok("default")
    }
}

#[allow(dead_code)]
pub struct BaseNode {
    params: Params,
    next_nodes: HashMap<String, Arc<dyn Node>>,
}

impl BaseNode {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            next_nodes: HashMap::new(),
        }
    }

    pub fn add_next(&mut self, action: String, node: Arc<dyn Node>) {
        self.next_nodes.insert(action, node);
    }
}

#[async_trait]
impl Node for BaseNode {
    
    #[allow(unused_variables)]
    async fn execute(&self, context: &Context) -> Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }
}

#[allow(dead_code)]
pub struct BatchNode {
    base: BaseNode,
    batch_size: usize,
}

impl BatchNode {
    pub fn new(params: Params, batch_size: usize) -> Self {
        Self {
            base: BaseNode::new(params),
            batch_size,
        }
    }
}

#[async_trait]
impl Node for BatchNode {
    #[allow(unused_variables)]
    async fn execute(&self, context: &Context) -> Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }
} 