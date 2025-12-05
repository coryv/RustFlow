use async_trait::async_trait;
use crate::stream_engine::{StreamNode, DebugConfig};
use crate::schema::WorkflowLoader;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;

pub struct ExecuteWorkflowNode {
    pub workflow_path: String,
    pub inputs: Option<Value>,
}

impl ExecuteWorkflowNode {
    pub fn new(workflow_path: String, inputs: Option<Value>) -> Self {
        Self { workflow_path, inputs }
    }
}

#[async_trait]
impl StreamNode for ExecuteWorkflowNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(rx) = inputs.get_mut(0) {
            if let Some(tx) = outputs.first() {
                while let Some(data) = rx.recv().await {
                    // 1. Load Workflow
                    let content = std::fs::read_to_string(&self.workflow_path)
                        .map_err(|e| anyhow::anyhow!("Failed to read workflow file '{}': {}", self.workflow_path, e))?;
                    
                    let loader = WorkflowLoader::new();
                    let mut definition = loader.load(&content)
                        .map_err(|e| anyhow::anyhow!("Failed to parse workflow '{}': {}", self.workflow_path, e))?;
                    
                    // 2b. Inject Capture Node for Return
                    // Find any node of type "return"
                    let return_node_id = definition.nodes.iter()
                        .find(|n| n.node_type == "return")
                        .map(|n| n.id.clone());

                    if let Some(ret_id) = return_node_id {
                        // Add a dummy capture node
                        definition.nodes.push(crate::schema::NodeDefinition {
                            id: "_capture_".to_string(),
                            node_type: "accumulate".to_string(), // Use accumulate as a dummy sink
                            config: serde_json::Value::Null,
                            on_error: None,
                        });
                        // Add edge from return node to capture node
                        definition.edges.push(crate::schema::EdgeDefinition {
                            from: ret_id,
                            from_port: None,
                            to: "_capture_".to_string(),
                            to_port: None,
                        });
                    }

                    // 2. Create Executor
                    // TODO: Pass secrets? For now, empty.
                    let secrets = std::collections::HashMap::new();
                    let debug_config = DebugConfig { limit_records: None }; // No limit for sub-workflow by default
                    let mut executor = definition.to_executor(&secrets, debug_config)?;
                    
                    // 3. Inject Data
                    // Combine node config inputs with incoming data
                    let input_payload = if let Some(config_inputs) = &self.inputs {
                        // Merge? For now just use config inputs if present, else data
                        config_inputs.clone()
                    } else {
                        data.clone()
                    };
                    
                    // Inject into the trigger node.
                    // Prioritize "child_workflow_trigger", fallback to "manual_trigger" or "trigger".
                    let trigger_id = definition.nodes.iter()
                        .find(|n| n.node_type == "child_workflow_trigger")
                        .map(|n| n.id.clone())
                        .or_else(|| {
                            // Fallback to manual_trigger or just "trigger"
                            definition.nodes.iter()
                                .find(|n| n.node_type == "manual_trigger" || n.id == "trigger")
                                .map(|n| n.id.clone())
                        });

                    if let Some(tid) = trigger_id {
                        executor.inject_input(&tid, input_payload);
                    } else {
                        eprintln!("ExecuteWorkflowNode: No suitable trigger found in sub-workflow.");
                    }
                    
                    // 4. Run Workflow
                    let (event_tx, mut event_rx) = tokio::sync::broadcast::channel(100);
                    executor.set_event_sender(event_tx);
                    
                    let execution_handle = tokio::spawn(async move {
                        executor.run().await
                    });
                    
                    let mut final_result = Value::Null;
                    
                    // Listen for events
                    while let Ok(event) = event_rx.recv().await {
                        if let crate::schema::ExecutionEvent::EdgeData { from: _, to, value } = event {
                             if to == "_capture_" {
                                 final_result = value;
                             }
                        }
                    }
                    
                    // Wait for execution to finish
                    let _ = execution_handle.await??;
                    
                    // 5. Emit Output
                    if let Err(e) = tx.send(final_result).await {
                         eprintln!("ExecuteWorkflowNode: Failed to send output: {}", e);
                         break;
                    }
                }
            }
        }
        Ok(())
    }
}
