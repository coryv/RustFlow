use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{Value, json};
use anyhow::Result;

pub struct AccumulateNode;

impl AccumulateNode {
    pub fn new() -> Self {
        Self
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
                    // Emit the current state of the accumulator
                    let output = json!(accumulator);
                    if let Err(e) = tx.send(output).await {
                        eprintln!("AccumulateNode: Failed to send output: {}", e);
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}
