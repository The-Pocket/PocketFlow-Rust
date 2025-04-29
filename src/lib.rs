pub mod node;
pub mod flow;
pub mod context;
pub mod error;

pub use node::*;
pub use flow::*;
pub use error::*;
pub use context::Context;

pub type Result<T> = std::result::Result<T, Error>;
pub type Params = std::collections::HashMap<String, serde_json::Value>; 