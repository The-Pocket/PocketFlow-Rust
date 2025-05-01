# PocketFlow-rs

A Rust implementation of [PocketFlow](https://github.com/The-Pocket/PocketFlow), a minimalist flow-based programming framework.

## Features

- **Lightweight**: Minimal dependencies, focused on core functionality
- **Flow-based Programming**: Build complex workflows with simple node connections
- **Async Support**: Built on top of async/await for efficient execution
- **Type Safety**: Leverage Rust's type system for robust flow definitions
- **Macro Support**: Easy flow definition with declarative macros

## Quick Start

```rust
use pocketflow_rs::{Context, Flow, Node, build_flow};
use serde_json::Value;

// Define your nodes
struct MyNode;

#[async_trait::async_trait]
impl Node for MyNode {
    async fn execute(&self, context: &Context) -> pocketflow_rs::Result<Value> {
        // Your node logic here
        Ok(Value::Null)
    }
}

// Create and run a flow
let flow = build_flow!(
    start: ("start", MyNode),
    nodes: [
        ("node1", MyNode),
        ("node2", MyNode)
    ],
    edges: [
        ("start", "node1"),
        ("node1", "node2", "condition")
    ]
);

let mut context = Context::new();
flow.run(context).await?;
```

## Core Concepts

### Node

A `Node` is the basic building block of a flow. It implements the `Node` trait:

```rust
#[async_trait::async_trait]
pub trait Node: Send + Sync {
    async fn execute(&self, context: &Context) -> Result<Value>;
    async fn prepare(&self, context: &mut Context) -> Result<()> {
        Ok(())
    }
    async fn post_process(&self, context: &mut Context, result: &Value) -> Result<&str> {
        Ok("default")
    }
}
```

### Flow

A `Flow` represents a directed graph of nodes with transitions between them. It can be created using the `build_flow!` macro:

```rust
let flow = build_flow!(
    start: ("start", start_node),
    nodes: [
        ("node1", node1),
        ("node2", node2)
    ],
    edges: [
        ("start", "node1"),  // Default transition
        ("node1", "node2", "condition")  // Conditional transition
    ]
);
```

### Context

The `Context` holds the state that flows through the nodes:

```rust
let mut context = Context::new();
context.set("key", Value::String("value".to_string()));
let value = context.get("key");
```

## Examples

See the `examples` directory for more usage examples:

- `basic.rs`: A simple flow with conditional transitions

## License

MIT License - see LICENSE file for details

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 