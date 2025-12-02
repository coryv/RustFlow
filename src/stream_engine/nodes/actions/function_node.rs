use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;
use std::sync::Arc;

pub struct FunctionNode {
    func: Arc<dyn Fn(Value) -> Result<Value> + Send + Sync>,
}

impl FunctionNode {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(Value) -> Result<Value> + Send + Sync + 'static,
    {
        Self {
            func: Arc::new(f),
        }
    }
}

#[async_trait]
impl StreamNode for FunctionNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(rx) = inputs.get_mut(0) {
            if let Some(tx) = outputs.get(0) {
                while let Some(data) = rx.recv().await {
                    let result = (self.func)(data)?;
                    tx.send(result).await?;
                }
            }
        }
        Ok(())
    }
}
