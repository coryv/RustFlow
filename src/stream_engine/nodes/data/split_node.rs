use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;

pub struct SplitNode {
    path: Option<String>,
}

impl SplitNode {
    pub fn new(path: Option<String>) -> Self {
        Self { path }
    }
}

#[async_trait]
impl StreamNode for SplitNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(rx) = inputs.get_mut(0) {
            if let Some(tx) = outputs.first() {
                while let Some(data) = rx.recv().await {
                    let items = if let Some(path) = &self.path {
                        // Use JMESPath
                        let expr = jmespath::compile(path).map_err(|e| anyhow::anyhow!("Invalid JMESPath: {}", e))?;
                        let result = expr.search(&data).map_err(|e| anyhow::anyhow!("JMESPath search failed: {}", e))?;
                        
                        let result_val = serde_json::to_value(&*result)?;
                        
                        if let Some(arr) = result_val.as_array() {
                            arr.clone()
                        } else {
                            vec![result_val]
                        }
                    } else {
                        // Assume root is array
                        if let Some(arr) = data.as_array() {
                            arr.clone()
                        } else {
                            vec![data] // Pass through single item if not array
                        }
                    };

                    for item in items {
                        if let Err(e) = tx.send(item).await {
                            eprintln!("SplitNode: Failed to send item: {}", e);
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
