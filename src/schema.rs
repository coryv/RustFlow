use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub nodes: Vec<NodeDefinition>,
    pub edges: Vec<EdgeDefinition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeDefinition {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub config: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EdgeDefinition {
    pub from: String,
    pub from_port: Option<String>,
    pub to: String,
    pub to_port: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionEvent {
    NodeStart { node_id: String },
    NodeFinish { node_id: String },
    EdgeData { from: String, to: String, value: Value },
    NodeError { node_id: String, error: String },
}

use crate::stream_engine::{StreamExecutor, DebugConfig};

impl WorkflowDefinition {
    pub fn to_executor(&self, secrets: &std::collections::HashMap<String, String>, debug_config: DebugConfig) -> Result<StreamExecutor> {
        let mut executor = StreamExecutor::new(debug_config);
        let factory = crate::stream_engine::factory::NodeFactory::new();

        for node_def in &self.nodes {
            let node = factory.create(&node_def.node_type, node_def.config.clone(), secrets)?;
            executor.add_node(node_def.id.clone(), node);
        }

        let registry = crate::node_registry::get_node_registry();
        let node_type_map: std::collections::HashMap<String, String> = self.nodes.iter()
            .map(|n| (n.id.clone(), n.node_type.clone()))
            .collect();

        for edge_def in &self.edges {
            let from_port = if let Some(p_str) = &edge_def.from_port {
                if let Ok(idx) = p_str.parse::<usize>() {
                    idx
                } else {
                    // Resolve named port
                    let node_type_id = node_type_map.get(&edge_def.from).ok_or_else(|| anyhow::anyhow!("Unknown node: {}", edge_def.from))?;
                    let node_type = registry.iter().find(|n| &n.id == node_type_id).ok_or_else(|| anyhow::anyhow!("Unknown node type: {}", node_type_id))?;
                    node_type.outputs.iter().position(|name| name == p_str).ok_or_else(|| anyhow::anyhow!("Unknown output port '{}' for node type '{}'", p_str, node_type_id))?
                }
            } else {
                0
            };

            let to_port = edge_def.to_port.as_deref().and_then(|p| p.parse::<usize>().ok()).unwrap_or(0);
            executor.add_connection(edge_def.from.clone(), from_port, edge_def.to.clone(), to_port);
        }

        Ok(executor)
    }
}

pub struct WorkflowLoader;

impl Default for WorkflowLoader {
    fn default() -> Self {
        Self::new()
    }
}

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
