use crate::{Context, Node, Result};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Transition {
    pub from_node: String,
    pub action: String,
    pub to_node: String,
}

pub struct Flow {
    start_node: Arc<dyn Node>,
    nodes: HashMap<String, Arc<dyn Node>>,
    transitions: Vec<Transition>,
}

impl Flow {
    pub fn new(start_node: Arc<dyn Node>) -> Self {
        Self {
            start_node,
            nodes: HashMap::new(),
            transitions: Vec::new(),
        }
    }

    pub fn add_node(&mut self, name: &str, node: Arc<dyn Node>) {
        self.nodes.insert(name.to_string(), node);
    }

    pub fn add_default_transition(&mut self, from: &str, to: &str) {
        self.transitions.push(Transition {
            from_node: from.to_string(),
            action: "default".to_string(),
            to_node: to.to_string(),
        });
    }

    pub fn add_transition(&mut self, from: &str, action: &str, to: &str) {
        self.transitions.push(Transition {
            from_node: from.to_string(),
            action: action.to_string(),
            to_node: to.to_string(),
        });
    }

    fn get_next_node(&self, current_node: &str, action: &str) -> Option<Arc<dyn Node>> {
        self.transitions
            .iter()
            .find(|t| t.from_node == current_node && t.action == action)
            .and_then(|t| self.nodes.get(&t.to_node).cloned())
    }

    pub async fn run(&self, mut context: Context) -> Result<()> {
        info!("Starting flow execution");
        
        let mut current_node = self.start_node.clone();
        let mut current_node_name = "start".to_string();
        
        loop {
            let node = current_node.clone();
            node.prepare(&mut context).await?;
            
            let result = node.execute(&context).await?;
            
            let action = node.post_process(&mut context, &result).await?;
            
            if let Some(next_node) = self.get_next_node(&current_node_name, &action) {
                current_node = next_node;
                current_node_name = self.transitions
                    .iter()
                    .find(|t| t.from_node == current_node_name && t.action == action)
                    .map(|t| t.to_node.clone())
                    .unwrap_or_else(|| "end".to_string());
            } else {
                info!("No next node found for action: {}", action);
                break;
            }
        }
        
        info!("Flow execution completed");
        Ok(())
    }
}

pub struct BatchFlow {
    flow: Flow,
    batch_size: usize,
}

impl BatchFlow {
    pub fn new(start_node: Arc<dyn Node>, batch_size: usize) -> Self {
        Self {
            flow: Flow::new(start_node),
            batch_size,
        }
    }

    pub async fn run_batch(&self, contexts: Vec<Context>) -> Result<()> {
        info!("Starting batch flow execution with {} items", contexts.len());
        
        for context in contexts {
            self.flow.run(context).await?;
        }
        
        info!("Batch flow execution completed");
        Ok(())
    }
}