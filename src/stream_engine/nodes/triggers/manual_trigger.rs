use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{json, Value};
use anyhow::Result;

pub struct ManualTrigger;

#[async_trait]
impl StreamNode for ManualTrigger {
    async fn run(&self, _inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        // Send one message to the first output and finish
        if let Some(tx) = outputs.first() {
            tx.send(json!(null)).await?;
        }
        Ok(())
    }
}
