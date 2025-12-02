use async_trait::async_trait;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;
use std::time::Duration;
use crate::stream_engine::StreamNode;

#[derive(Debug)]
pub struct DelayNode {
    duration_ms: u64,
}

impl DelayNode {
    pub fn new(config: Value) -> Self {
        let duration_ms = config.get("duration_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000); // Default to 1 second
            
        Self { duration_ms }
    }
}

#[async_trait]
impl StreamNode for DelayNode {
    async fn run(&self, inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        let mut input = inputs.into_iter().next().unwrap();
        let output = outputs.into_iter().next().unwrap();

        while let Some(value) = input.recv().await {
            tokio::time::sleep(Duration::from_millis(self.duration_ms)).await;
            if output.send(value).await.is_err() {
                break;
            }
        }

        Ok(())
    }
}
