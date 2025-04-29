use pocketflow_rs::{Context, Flow, Node};
use serde_json::Value;
use std::sync::Arc;
use rand::Rng;

// A simple node that prints a message
struct PrintNode {
    message: String,
}

impl PrintNode {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Node for PrintNode {
    async fn execute(&self, context: &Context) -> pocketflow_rs::Result<Value> {
        println!("PrintNode: {}", self.message);
        Ok(Value::String(self.message.clone()))
    }

}


// A node that generates a random number
struct RandomNumberNode {
    max: i64,
}

impl RandomNumberNode {
    fn new(max: i64) -> Self {
        Self { max }
    }
}

#[async_trait::async_trait]
impl Node for RandomNumberNode {
    async fn execute(&self, context: &Context) -> pocketflow_rs::Result<Value> {
        let num = rand::thread_rng().gen_range(0..self.max);
        println!("RandomNumberNode: Generated number {}", num);
        Ok(Value::Number(num.into()))
    }

    async fn post_process(&self, context: &mut Context, result: &Value) -> pocketflow_rs::Result<&str> {
        let num = result.as_i64().unwrap_or(0);
        context.set("number", Value::Number(num.into()));
        // Return different actions based on the number
        if num < self.max / 3 {
            Ok("small")
        } else if num < 2 * self.max / 3 {
            Ok("medium")
        } else {
            Ok("large")
        }
    }
}

// A node that processes small numbers
struct SmallNumberNode;

#[async_trait::async_trait]
impl Node for SmallNumberNode {
    async fn execute(&self, context: &Context) -> pocketflow_rs::Result<Value> {
        let num = context.get("number")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        println!("SmallNumberNode: Processing small number {}", num);
        Ok(Value::String(format!("Small number processed: {}", num)))
    }
}

// A node that processes medium numbers
struct MediumNumberNode;

#[async_trait::async_trait]
impl Node for MediumNumberNode {
    async fn execute(&self, context: &Context) -> pocketflow_rs::Result<Value> {
        let num = context.get("number")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        println!("MediumNumberNode: Processing medium number {}", num);
        Ok(Value::String(format!("Medium number processed: {}", num)))
    }
}

// A node that processes large numbers
struct LargeNumberNode;

#[async_trait::async_trait]
impl Node for LargeNumberNode {
    async fn execute(&self, context: &Context) -> pocketflow_rs::Result<Value> {
        let num = context.get("number")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        println!("LargeNumberNode: Processing large number {}", num);
        Ok(Value::String(format!("Large number processed: {}", num)))
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {

    // Create nodes
    let begin_node = Arc::new(PrintNode::new("Begin Node"));
    let random_node = Arc::new(RandomNumberNode::new(100));
    let small_node = Arc::new(SmallNumberNode);
    let medium_node = Arc::new(MediumNumberNode);
    let large_node = Arc::new(LargeNumberNode);
    
    // Create flow with random node as start
    let mut flow = Flow::new(begin_node);
    
  
    // Add nodes to flow
    flow.add_node("small", small_node);
    flow.add_node("medium", medium_node);
    flow.add_node("large", large_node);
    flow.add_node("rand", random_node);
    
    
    // Add transitions based on conditions
    flow.add_default_transition("start", "rand");
    flow.add_transition("rand", "small", "small");
    flow.add_transition("rand", "medium", "medium");
    flow.add_transition("rand", "large", "large");
    
    // Create context
    let mut context = Context::new();
    
    // Run the flow
    println!("Starting flow execution...");
    flow.run(context).await?;
    println!("Flow execution completed!");
    
    Ok(())
}