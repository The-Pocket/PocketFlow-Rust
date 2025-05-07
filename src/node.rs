use crate::{Params, context::Context};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

pub trait ProcessState: Send + Sync {
    fn is_default(&self) -> bool;
    fn to_condition(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum BaseState {
    Success,
    Failure,
    #[default]
    Default,
}

impl ProcessState for BaseState {
    fn is_default(&self) -> bool {
        matches!(self, BaseState::Default)
    }

    fn to_condition(&self) -> String {
        match self {
            BaseState::Success => "success".to_string(),
            BaseState::Failure => "failure".to_string(),
            BaseState::Default => "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessResult<S: ProcessState> {
    pub state: S,
    pub message: String,
}

impl<S: ProcessState> ProcessResult<S> {
    pub fn new(state: S, message: String) -> Self {
        Self { state, message }
    }
}

impl<S: ProcessState + Default> Default for ProcessResult<S> {
    fn default() -> Self {
        Self {
            state: S::default(),
            message: "default".to_string(),
        }
    }
}

#[async_trait]
pub trait Node: Send + Sync {
    type State: ProcessState + Default;

    #[allow(unused_variables)]
    async fn prepare(&self, context: &mut Context) -> Result<()> {
        Ok(())
    }

    async fn execute(&self, context: &Context) -> Result<serde_json::Value>;

    #[allow(unused_variables)]
    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<serde_json::Value>,
    ) -> Result<ProcessResult<Self::State>> {
        match result {
            Ok(value) => {
                context.set("result", value.clone());
                Ok(ProcessResult::default())
            }
            Err(e) => {
                context.set("error", serde_json::Value::String(e.to_string()));
                Ok(ProcessResult::new(Self::State::default(), e.to_string()))
            }
        }
    }
}

pub trait BaseNodeTrait: Node<State = BaseState> {}

#[allow(dead_code)]
pub struct BaseNode {
    params: Params,
    next_nodes: HashMap<String, Arc<dyn BaseNodeTrait>>,
}

impl BaseNode {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            next_nodes: HashMap::new(),
        }
    }

    pub fn add_next(&mut self, action: String, node: Arc<dyn BaseNodeTrait>) {
        self.next_nodes.insert(action, node);
    }
}

#[async_trait]
impl Node for BaseNode {
    type State = BaseState;

    #[allow(unused_variables)]
    async fn execute(&self, context: &Context) -> Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }
}

impl BaseNodeTrait for BaseNode {}

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
    type State = BaseState;

    #[allow(unused_variables)]
    async fn execute(&self, context: &Context) -> Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }
}

impl BaseNodeTrait for BatchNode {}
