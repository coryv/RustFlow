use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;

pub struct ConsoleOutputNode;

#[async_trait]
impl StreamNode for ConsoleOutputNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, _outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(mut rx) = inputs.pop() {
            while let Some(value) = rx.recv().await {
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
        }
        Ok(())
    }
}
