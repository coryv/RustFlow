use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;

pub struct SetData {
    data: Value,
}

impl SetData {
    pub fn new(data: Value) -> Self {
        Self { data }
    }
}

#[async_trait]
impl StreamNode for SetData {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        // Wait for input trigger, then send data
        if let Some(rx) = inputs.get_mut(0) {
            if let Some(tx) = outputs.get(0) {
                while let Some(_) = rx.recv().await {
                    tx.send(self.data.clone()).await?;
                }
            }
        }
        Ok(())
    }
}
