use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub nodes: Vec<NodeDefinition>,
    pub edges: Vec<EdgeDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default, alias = "config")]
    pub data: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EdgeDefinition {
    pub from: String,
    #[serde(default)]
    pub from_port: usize,
    pub to: String,
    #[serde(default)]
    pub to_port: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionEvent {
    NodeStart { node_id: String },
    NodeFinish { node_id: String },
    EdgeData { from: String, to: String, value: Value },
    NodeError { node_id: String, error: String },
}

use crate::stream_engine::StreamExecutor;

impl WorkflowDefinition {
    pub fn to_executor(&self, secrets: &std::collections::HashMap<String, String>) -> Result<StreamExecutor> {
        let mut executor = StreamExecutor::new();
        let factory = crate::stream_engine::factory::NodeFactory::new();

        for node_def in &self.nodes {
            let node = factory.create(&node_def.node_type, node_def.data.clone(), secrets)?;
            executor.add_node(node_def.id.clone(), node);
        }

        for edge_def in &self.edges {
            executor.add_connection(edge_def.from.clone(), edge_def.from_port, edge_def.to.clone(), edge_def.to_port);
        }

        Ok(executor)
    }
}

pub struct WorkflowLoader;

impl WorkflowLoader {
    pub fn new() -> Self {
        Self
    }

    pub fn load(&self, content: &str) -> Result<WorkflowDefinition> {
        // Try parsing as YAML (which is a superset of JSON)
        let def: WorkflowDefinition = serde_yaml::from_str(content)?;
        Ok(def)
    }
}
