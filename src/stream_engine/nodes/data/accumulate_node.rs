use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{Value, json};
use anyhow::Result;

pub struct AccumulateNode {
    batch_size: Option<usize>,
}

impl AccumulateNode {
    pub fn new(config: Value) -> Self {
        let batch_size = config.get("batch_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .filter(|&v| v > 0);
        Self { batch_size }
    }
}

#[async_trait]
impl StreamNode for AccumulateNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(rx) = inputs.get_mut(0) {
            if let Some(tx) = outputs.first() {
                let mut accumulator = Vec::new();
                
                while let Some(data) = rx.recv().await {
                    accumulator.push(data);
                    
                    if let Some(limit) = self.batch_size {
                        if accumulator.len() >= limit {
                             let batch = std::mem::take(&mut accumulator);
                             tx.send(json!(batch)).await?;
                        }
                    }
                }
                
                // Stream ended. If there is leftover data or if we are in "Collect All" mode
                // We emit the remaining accumulator.
                // NOTE: If batch_size was set and we have 0 items left (perfect split), we typically don't emit empty array?
                // Or "Collect All" with 0 items?
                // Let's emit only if not empty.
                if !accumulator.is_empty() {
                    tx.send(json!(accumulator)).await?;
                }
            }
        }
        Ok(())
    }
}
