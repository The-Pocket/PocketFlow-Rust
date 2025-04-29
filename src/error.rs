use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Node execution error: {0}")]
    NodeExecution(String),

    #[error("Invalid node transition: {0}")]
    InvalidTransition(String),

    #[error("Context error: {0}")]
    Context(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
} 