use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;

pub struct SetDataNode {
    data: Value,
}

impl SetDataNode {
    pub fn new(data: Value) -> Self {
        Self { data }
    }
}

#[async_trait]
impl StreamNode for SetDataNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        // Wait for input trigger, then send data
        if let Some(rx) = inputs.get_mut(0) {
            if let Some(tx) = outputs.first() {
                while (rx.recv().await).is_some() {
                    tx.send(self.data.clone()).await?;
                }
            }
        }
        Ok(())
    }
}
