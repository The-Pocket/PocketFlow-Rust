#![cfg(feature = "debug")]
use std::fmt::Debug;

pub trait DebugVisualizer {
    fn visualize<T: Debug>(&self, data: &T) -> String;
    fn visualize_flow(&self, flow_data: &[u8]) -> String;
}

pub struct ConsoleDebugVisualizer;

impl DebugVisualizer for ConsoleDebugVisualizer {
    fn visualize<T: Debug>(&self, data: &T) -> String {
        format!("{:?}", data)
    }

    #[allow(unused_variables)]
    fn visualize_flow(&self, flow_data: &[u8]) -> String {
        // TODO: Implement flow visualization
        "Flow visualization not implemented".to_string()
    }
}

pub struct GraphDebugVisualizer;

impl DebugVisualizer for GraphDebugVisualizer {
    fn visualize<T: Debug>(&self, data: &T) -> String {
        // TODO: Implement graph visualization
        format!("Graph visualization of {:?}", data)
    }

    #[allow(unused_variables)]
    fn visualize_flow(&self, flow_data: &[u8]) -> String {
        // TODO: Implement flow graph visualization
        "Flow graph visualization not implemented".to_string()
    }
}
