use crate::{context::Context, node::{Node, ProcessState}};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

pub struct Flow<S: ProcessState + Default> {
    nodes: HashMap<String, Arc<dyn Node<State = S>>>,
    edges: HashMap<String, Vec<(String, String)>>, // (to_node, condition)
    start_node: String,
}

impl<S: ProcessState + Default> Flow<S> {
    pub fn new(start_node_name: &str, start_node: Arc<dyn Node<State = S>>) -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(start_node_name.to_string(), start_node);
        
        Self {
            nodes,
            edges: HashMap::new(),
            start_node: start_node_name.to_string(),
        }
    }

    pub fn add_node(&mut self, name: &str, node: Arc<dyn Node<State = S>>) {
        self.nodes.insert(name.to_string(), node);
    }

    pub fn add_edge(&mut self, from: &str, to: &str, condition: S) {
        self.edges.entry(from.to_string()).or_insert_with(Vec::new).push((to.to_string(), condition.to_condition()));
    }
    
    pub async fn run(&self, mut context: Context) -> Result<Value> {
        let mut current_node = self.start_node.clone();
        
        while let Some(node) = self.nodes.get(&current_node) {
            // Prepare
            info!("Preparing node: {}", current_node);
            node.prepare(&mut context).await?;
            
            // Execute
            info!("Executing node: {}", current_node);
            let result = node.execute(&context).await;
            
            // Post process
            info!("Post processing node: {}", current_node);
            let process_result = node.post_process(&mut context, &result).await?;
            
            // Find next node based on the state returned by post_process
            if let Some(edges) = self.edges.get(&current_node) {
                // Get the condition from the node state
                let condition = process_result.state.to_condition();
                
                // Try to find an edge matching the condition
                let next_node_info = edges.iter().find(|(_, edge_condition)| {
                    edge_condition == &condition
                });

                if let Some((next, _)) = next_node_info {
                    current_node = next.clone();
                } else {
                    // If no matching edge found, try the default condition
                    let default_edge = edges.iter().find(|(_, edge_condition)| {
                        edge_condition == "default"
                    });

                    if let Some((next, _)) = default_edge {
                        current_node = next.clone();
                    } else {
                        info!("No edge found for node '{}' with condition '{}'. Stopping flow.", current_node, condition);
                        break;
                    }
                }
            } else {
                info!("Node '{}' has no outgoing edges. Stopping flow.", current_node);
                break;
            }
        }
        
        Ok(context.get("result").unwrap_or(&Value::Null).clone())
    }
}

#[allow(dead_code)]
pub struct BatchFlow<S: ProcessState + Default> {
    flow: Flow<S>,
    batch_size: usize,
}

impl<S: ProcessState + Default> BatchFlow<S> {
    pub fn new(start_node_name: &str, start_node: Arc<dyn Node<State = S>>, batch_size: usize) -> Self {
        Self {
            flow: Flow::new(start_node_name, start_node),
            batch_size,
        }
    }

    pub async fn run_batch(&self, contexts: Vec<Context>) -> Result<()> {
        info!(
            "Starting batch flow execution with {} items",
            contexts.len()
        );

        for context in contexts {
            self.flow.run(context).await?;
        }

        info!("Batch flow execution completed");
        Ok(())
    }
}

#[macro_export]
macro_rules! build_flow {
    (start: ($name: expr, $node:expr)) => {{
        $crate::flow::Flow::new($name, std::sync::Arc::new($node))
    }};

    (
        start: ($start_name:expr, $start_node:expr),
        nodes: [$(($name:expr, $node:expr)),* $(,)?]
    ) => {{
        let mut g = $crate::flow::Flow::new($start_name, std::sync::Arc::new($start_node));
        $(
            g.add_node($name, std::sync::Arc::new($node));
        )*
        g
    }};

    // Complete version with proper-edge handling
    (
        start: ($start_name:expr, $start_node:expr),
        nodes: [$(($name:expr, $node:expr)),* $(,)?],
        edges: [
            $($edge:tt),* $(,)?
        ]
    ) => {{
        let mut g = $crate::flow::Flow::new($start_name, std::sync::Arc::new($start_node));
        // Add all nodes first
        $(
            g.add_node($name, std::sync::Arc::new($node));
        )*
        // Handle edges appropriately
        $(
            build_flow!(@edge g, $edge);
        )*
        g
    }};


    (@edge $g:expr, ($from:expr, $to:expr, $condition:expr)) => {
        $g.add_edge($from, $to, $condition);
    };
}

#[macro_export]
macro_rules! build_batch_flow {
    (start: ($name: expr, $node:expr), batch_size: $batch_size:expr) => {{
        BatchFlow::new($name, std::sync::Arc::new($node), $batch_size)
    }};

    (
        start: ($start_name:expr, $start_node:expr),
        nodes: [$(($name:expr, $node:expr)),* $(,)?],
        batch_size: $batch_size:expr
    ) => {{
        let mut g = BatchFlow::new($start_name, std::sync::Arc::new($start_node), $batch_size);
        $(
            g.flow.add_node($name, std::sync::Arc::new($node));
        )*
        g
    }};

    // Complete version with proper-edge handling
    (
        start: ($start_name:expr, $start_node:expr),
        nodes: [$(($name:expr, $node:expr)),* $(,)?],
        edges: [
            $($edge:tt),* $(,)?
        ],
        batch_size: $batch_size:expr
    ) => {{
        let mut g = BatchFlow::new($start_name, std::sync::Arc::new($start_node), $batch_size);
        // Add all nodes first
        $(
            g.flow.add_node($name, std::sync::Arc::new($node));
        )*
        // Handle edges appropriately
        $(
            build_flow!(@edge g.flow, $edge);
        )*
        g
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Node, ProcessState, ProcessResult};
    use serde_json::json;
    use async_trait::async_trait;

    #[derive(Debug, Clone, PartialEq)]
    #[allow(dead_code)]
    enum CustomState {
        Success,
        Failure,
        Default,
    }

    impl ProcessState for CustomState {
        fn is_default(&self) -> bool {
            matches!(self, CustomState::Default)
        }

        fn to_condition(&self) -> String {
            match self {
                CustomState::Success => "success".to_string(),
                CustomState::Failure => "failure".to_string(),
                CustomState::Default => "default".to_string(),
            }
        }
    }

    impl Default for CustomState {
        fn default() -> Self {
            CustomState::Default
        }
    }

    struct TestNode {
        result: Value,
        state: CustomState,
    }

    impl TestNode {
        fn new(result: Value, state: CustomState) -> Self {
            Self { result, state }
        }
    }

    #[async_trait]
    impl Node for TestNode {
        type State = CustomState;

        async fn execute(&self, _context: &Context) -> Result<Value> {
            Ok(self.result.clone())
        }

        async fn post_process(&self, context: &mut Context, result: &Result<Value>) -> Result<ProcessResult<CustomState>> {
            match result {
                Ok(value) => {
                    context.set("result", value.clone());
                    Ok(ProcessResult::new(self.state.clone(), "test".to_string()))
                }
                Err(e) => {
                    context.set("error", json!(e.to_string()));
                    Ok(ProcessResult::new(CustomState::Default, e.to_string()))
                }
            }
        }
    }

    #[tokio::test]
    async fn test_flow_with_custom_state() {
        let node1 = Arc::new(TestNode::new(
            json!({"data": "test1"}),
            CustomState::Success,
        ));
        let node2 = Arc::new(TestNode::new(
            json!({"data": "test2"}),
            CustomState::Default,
        ));
        let end_node = Arc::new(TestNode::new(
            json!({"final_result": "finished"}),
            CustomState::Default,
        ));

        let mut flow = Flow::<CustomState>::new("start", node1);
        flow.add_node("next", node2);
        flow.add_node("end", end_node);

        flow.add_edge("start", "next", CustomState::Success);
        flow.add_edge("next", "end", CustomState::Default);

        let context = Context::new();
        let result = flow.run(context).await.unwrap();

        assert_eq!(result, json!({"final_result": "finished"}));
    }

    #[tokio::test]
    async fn test_batch_flow() {
        let node1 = TestNode::new(
            json!({"data": "test1"}),
            CustomState::Success,
        );
        let node2 = TestNode::new(
            json!({"data": "test2"}),
            CustomState::Default,
        );
        
        let mut batch_flow = BatchFlow::<CustomState>::new("start", Arc::new(node1), 10);
        batch_flow.flow.add_node("next", Arc::new(node2));
        batch_flow.flow.add_edge("start", "next", CustomState::Success);
        batch_flow.flow.add_edge("next", "end", CustomState::Default);
        
        let contexts = vec![Context::new(), Context::new()];
        batch_flow.run_batch(contexts).await.unwrap();
    }

    #[tokio::test]
    async fn test_build_flow_macro() {
        // Test basic flow with start node only
        let node1 = TestNode::new(
            json!({"data": "test1"}),
            CustomState::Success,
        );
        let flow1 = build_flow!(
            start: ("start", node1)
        );
        let context = Context::new();
        let result = flow1.run(context).await.unwrap();
        assert_eq!(result, json!({"data": "test1"}));

        // Test flow with multiple nodes
        let node1 = TestNode::new(
            json!({"data": "test1"}),
            CustomState::Success,
        );
        let node2 = TestNode::new(
            json!({"data": "test2"}),
            CustomState::Default,
        );
        let end_node = TestNode::new(
            json!({"final_result": "finished"}),
            CustomState::Default,
        );
        let flow2 = build_flow!(
            start: ("start", node1),
            nodes: [("next", node2), ("end", end_node)],
            edges: [
                ("start", "next", CustomState::Success),
                ("next", "end", CustomState::Default)
            ]
        );
        let context = Context::new();
        let result = flow2.run(context).await.unwrap();
        assert_eq!(result, json!({"final_result": "finished"}));

        // Test flow with default edges
        let node1 = TestNode::new(
            json!({"data": "test1"}),
            CustomState::Success,
        );
        let node2 = TestNode::new(
            json!({"data": "test2"}),
            CustomState::Default,
        );
        let flow3 = build_flow!(
            start: ("start", node1),
            nodes: [("next", node2)],
            edges: [
                ("start", "next", CustomState::Default)
            ]
        );
        let context = Context::new();
        let result = flow3.run(context).await.unwrap();
        assert_eq!(result, json!({"data": "test2"}));
    }
}