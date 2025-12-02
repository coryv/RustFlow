use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;

pub struct RouterNode {
    pub key: String,
    pub value: Value,
}

impl RouterNode {
    pub fn new(key: String, value: Value) -> Self {
        Self { key, value }
    }
}

#[async_trait]
impl StreamNode for RouterNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(rx) = inputs.get_mut(0) {
            while let Some(data) = rx.recv().await {
                // Check condition
                let match_found = data.get(&self.key) == Some(&self.value);
                
                if match_found {
                    // Send to Port 0 (True)
                    if let Some(tx) = outputs.get(0) {
                        tx.send(data).await?;
                    }
                } else {
                    // Send to Port 1 (False)
                    if let Some(tx) = outputs.get(1) {
                        tx.send(data).await?;
                    }
                }
            }
        }
        Ok(())
    }
}
