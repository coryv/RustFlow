use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;

pub struct ConsoleOutput;

#[async_trait]
impl StreamNode for ConsoleOutput {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(rx) = inputs.get_mut(0) {
            while let Some(data) = rx.recv().await {
                println!("ConsoleOutput: {:#?}", data);
                // Pass through if there is an output connected
                if let Some(tx) = outputs.get(0) {
                    tx.send(data).await?;
                }
            }
        }
        Ok(())
    }
}
