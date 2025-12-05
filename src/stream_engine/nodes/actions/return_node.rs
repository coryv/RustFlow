use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;

pub struct ReturnNode {
    pub value: Option<Value>,
}

impl ReturnNode {
    pub fn new(value: Option<Value>) -> Self {
        Self { value }
    }
}



#[async_trait]
impl StreamNode for ReturnNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(rx) = inputs.get_mut(0) {
            while let Some(data) = rx.recv().await {
                let result = if let Some(val) = &self.value {
                    match val {
                        Value::String(s) => {
                            let mut env = crate::stream_engine::expressions::create_environment();
                            match env.render_str(s, &data) {
                                Ok(rendered) => {
                                    // Try to parse as JSON to preserve types (e.g. "20" -> 20)
                                    if let Ok(parsed) = serde_json::from_str::<Value>(&rendered) {
                                        parsed
                                    } else {
                                        Value::String(rendered)
                                    }
                                },
                                Err(e) => {
                                    eprintln!("Template rendering failed: {}", e);
                                    Value::String(s.clone())
                                }
                            }
                        },
                        _ => val.clone(),
                    }
                } else {
                    data
                };
                
                // We print it so the Executor can capture it via the dummy edge we injected
                // Actually, we just need to send it?
                // Wait, ReturnNode doesn't have outputs connected in the original graph.
                // But ExecuteWorkflowNode injected a dummy edge from ReturnNode to _capture_.
                // So ReturnNode MUST write to its output!
                
                // In the original ReturnNode implementation, I didn't write to outputs!
                // I only printed.
                // I MUST write to outputs if they exist.
                
                if let Some(tx) = outputs.first() {
                    let _ = tx.send(result.clone()).await;
                }
                
                println!("Workflow Return: {:?}", result);
            }
        }
        Ok(())
    }
}
