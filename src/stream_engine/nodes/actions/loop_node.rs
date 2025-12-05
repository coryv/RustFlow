use async_trait::async_trait;
use crate::stream_engine::{StreamNode, DebugConfig};
use crate::schema::{WorkflowDefinition, NodeDefinition, EdgeDefinition, ExecutionEvent};
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::{Result, anyhow};
use std::fs;
use std::collections::HashMap;

pub struct LoopNode {
    workflow_path: String,
    max_iterations: usize,
    condition_key: String,
    condition_operator: String,
    condition_value: Value,
}

impl LoopNode {
    pub fn new(
        workflow_path: String,
        max_iterations: usize,
        condition_key: String,
        condition_operator: String,
        condition_value: Value,
    ) -> Self {
        Self {
            workflow_path,
            max_iterations,
            condition_key,
            condition_operator,
            condition_value,
        }
    }

    fn check_condition(&self, data: &Value) -> bool {
        let expr = match jmespath::compile(&self.condition_key) {
            Ok(e) => e,
            Err(_) => return false,
        };

        let search_result = match expr.search(data) {
            Ok(r) => r,
            Err(_) => return false,
        };

        let value = serde_json::to_value(search_result).unwrap_or(Value::Null);

        match self.condition_operator.as_str() {
            "==" => value == self.condition_value,
            "!=" => value != self.condition_value,
            ">" => {
                if let (Some(a), Some(b)) = (value.as_f64(), self.condition_value.as_f64()) {
                    a > b
                } else {
                    false
                }
            }
            "<" => {
                if let (Some(a), Some(b)) = (value.as_f64(), self.condition_value.as_f64()) {
                    a < b
                } else {
                    false
                }
            }
            ">=" => {
                if let (Some(a), Some(b)) = (value.as_f64(), self.condition_value.as_f64()) {
                    a >= b
                } else {
                    false
                }
            }
            "<=" => {
                if let (Some(a), Some(b)) = (value.as_f64(), self.condition_value.as_f64()) {
                    a <= b
                } else {
                    false
                }
            }
            "contains" => {
                if let Some(s) = value.as_str() {
                    if let Some(sub) = self.condition_value.as_str() {
                        return s.contains(sub);
                    }
                }
                false
            }
            _ => false,
        }
    }
}

#[async_trait]
impl StreamNode for LoopNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(rx) = inputs.get_mut(0) {
            while let Some(initial_input) = rx.recv().await {
                let mut current_input = initial_input.clone();
                let mut iteration = 0;

                loop {
                    if iteration >= self.max_iterations {
                        eprintln!("LoopNode: Max iterations ({}) reached.", self.max_iterations);
                        break;
                    }

                    // Load workflow definition
                    let content = fs::read_to_string(&self.workflow_path)
                        .map_err(|e| anyhow!("Failed to read workflow file: {}", e))?;
                    let mut definition: WorkflowDefinition = serde_yaml::from_str(&content)
                        .map_err(|e| anyhow!("Failed to parse workflow YAML: {}", e))?;

                    // Inject capture node logic (similar to ExecuteWorkflowNode)
                    let capture_node_id = "_capture_";
                    definition.nodes.push(NodeDefinition {
                        id: capture_node_id.to_string(),
                        node_type: "accumulate".to_string(), // Use accumulate to capture output
                        config: Value::Null,
                        on_error: None,
                    });

                    // Find return node and connect to capture
                    let return_node_id = definition.nodes.iter()
                        .find(|n| n.node_type == "return")
                        .map(|n| n.id.clone());

                    if let Some(rid) = return_node_id {
                        definition.edges.push(EdgeDefinition {
                            from: rid,
                            to: capture_node_id.to_string(),
                            from_port: None,
                            to_port: None,
                        });
                    } else {
                        return Err(anyhow!("Sub-workflow must have a 'return' node to be used in a loop."));
                    }

                    // Create Executor
                    let mut executor = definition.to_executor(&HashMap::new(), DebugConfig::default())?;
                    
                    // Inject input
                    // Prioritize "child_workflow_trigger", fallback to "manual_trigger" or "trigger".
                    let trigger_id = definition.nodes.iter()
                        .find(|n| n.node_type == "child_workflow_trigger")
                        .map(|n| n.id.clone())
                        .or_else(|| {
                            definition.nodes.iter()
                                .find(|n| n.node_type == "manual_trigger" || n.id == "trigger")
                                .map(|n| n.id.clone())
                        });

                    if let Some(tid) = trigger_id {
                        executor.inject_input(&tid, current_input.clone());
                    } else {
                        eprintln!("LoopNode: No suitable trigger found in sub-workflow.");
                    }

                    // Run workflow and capture output
                    let (event_tx, mut event_rx) = tokio::sync::broadcast::channel(100);
                    executor.set_event_sender(event_tx);

                    let handle = tokio::spawn(async move {
                        executor.run().await
                    });

                    let mut captured_result: Option<Value> = None;

                    // Listen for events to capture output
                    while let Ok(event) = event_rx.recv().await {
                        if let ExecutionEvent::EdgeData { to, value, .. } = event {
                            if to == capture_node_id {
                                // This is the data sent to our capture node
                                captured_result = Some(value);
                            }
                        }
                    }

                    handle.await??;

                    if let Some(result) = captured_result {
                        current_input = result.clone();
                        
                        // Check condition
                        if self.check_condition(&current_input) {
                            // Condition met (e.g. has_more == true), continue loop
                            iteration += 1;
                            continue;
                        } else {
                            // Condition not met (e.g. has_more == false), done
                            // Emit final result
                            if let Some(tx) = outputs.first() {
                                tx.send(current_input).await?;
                            }
                            break;
                        }
                    } else {
                        // No result captured?
                        eprintln!("LoopNode: No result captured from sub-workflow iteration.");
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}
