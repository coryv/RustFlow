use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{json, Value};
use anyhow::Result;

pub struct WebhookTrigger {
    path: String,
    method: String,
}

impl WebhookTrigger {
    pub fn new(path: String, method: String) -> Self {
        Self { path, method }
    }
}

#[async_trait]
impl StreamNode for WebhookTrigger {
    async fn run(&self, _inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        // In a real implementation, this would listen on a channel fed by an HTTP server.
        // For now, we'll simulate receiving one webhook event to kick off the workflow.
        if let Some(tx) = outputs.first() {
            let data = json!({
                "path": self.path,
                "method": self.method,
                "body": { "mock": "payload" },
                "query": {}
            });
            tx.send(data).await?;
        }
        Ok(())
    }
}
