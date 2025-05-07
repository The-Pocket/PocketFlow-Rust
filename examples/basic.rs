use anyhow::Result;
use pocketflow_rs::{Context, Node, ProcessResult, ProcessState, build_flow};
use rand::Rng;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Default)]
enum NumberState {
    Small,
    Medium,
    Large,
    #[default]
    Default,
}

impl ProcessState for NumberState {
    fn is_default(&self) -> bool {
        matches!(self, NumberState::Default)
    }

    fn to_condition(&self) -> String {
        match self {
            NumberState::Small => "small".to_string(),
            NumberState::Medium => "medium".to_string(),
            NumberState::Large => "large".to_string(),
            NumberState::Default => "default".to_string(),
        }
    }
}

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
    type State = NumberState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        println!("PrintNode: {}, Context: {}", self.message, context);
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
    type State = NumberState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let num = rand::thread_rng().gen_range(0..self.max);
        println!(
            "RandomNumberNode: Generated number {}, Context: {}",
            num, context
        );
        Ok(Value::Number(num.into()))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<NumberState>> {
        let num = result.as_ref().unwrap().as_i64().unwrap_or(0);
        context.set("number", Value::Number(num.into()));
        // Return different states based on the number
        let state = if num < self.max / 3 {
            NumberState::Small
        } else if num < 2 * self.max / 3 {
            NumberState::Medium
        } else {
            NumberState::Large
        };
        let condition = state.to_condition();
        Ok(ProcessResult::new(state, condition))
    }
}

// A node that processes small numbers
struct SmallNumberNode;

#[async_trait::async_trait]
impl Node for SmallNumberNode {
    type State = NumberState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let num = context.get("number").and_then(|v| v.as_i64()).unwrap_or(0);
        println!("SmallNumberNode: Processing small number {}", num);
        Ok(Value::String(format!("Small number processed: {}", num)))
    }
}

// A node that processes medium numbers
struct MediumNumberNode;

#[async_trait::async_trait]
impl Node for MediumNumberNode {
    type State = NumberState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let num = context.get("number").and_then(|v| v.as_i64()).unwrap_or(0);
        println!("MediumNumberNode: Processing medium number {}", num);
        Ok(Value::String(format!("Medium number processed: {}", num)))
    }
}

// A node that processes large numbers
struct LargeNumberNode;

#[async_trait::async_trait]
impl Node for LargeNumberNode {
    type State = NumberState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let num = context.get("number").and_then(|v| v.as_i64()).unwrap_or(0);
        println!("LargeNumberNode: Processing large number {}", num);
        Ok(Value::String(format!("Large number processed: {}", num)))
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Create nodes
    let begin_node = PrintNode::new("Begin Node");
    let random_node = RandomNumberNode::new(100);
    let small_node = SmallNumberNode;
    let medium_node = MediumNumberNode;
    let large_node = LargeNumberNode;

    // Create flow using macro
    let flow = build_flow!(
        start: ("start", begin_node),
        nodes: [
            ("rand", random_node),
            ("small", small_node),
            ("medium", medium_node),
            ("large", large_node)
        ],
        edges: [
            ("start", "rand", NumberState::Default),
            ("rand", "small", NumberState::Small),
            ("rand", "medium", NumberState::Medium),
            ("rand", "large", NumberState::Large)
        ]
    );

    // Create context
    let context = Context::new();

    // Run the flow
    println!("Starting flow execution...");
    flow.run(context).await?;
    println!("Flow execution completed!");

    Ok(())
}
