use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;
use std::collections::HashSet;

pub struct DedupeNode {
    key: Option<String>,
}

impl DedupeNode {
    pub fn new(key: Option<String>) -> Self {
        Self { key }
    }
}

#[async_trait]
impl StreamNode for DedupeNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if inputs.is_empty() || outputs.is_empty() {
            return Ok(());
        }

        let mut input = inputs.remove(0);
        let output = outputs.first().unwrap();
        
        let mut seen = HashSet::new();

        while let Some(value) = input.recv().await {
            let hash_key = if let Some(k) = &self.key {
                // Dedupe by specific key
                value.get(k).map(|v| v.to_string()).unwrap_or_else(|| "null".to_string())
            } else {
                // Dedupe by entire record
                value.to_string()
            };

            if !seen.contains(&hash_key) {
                seen.insert(hash_key);
                output.send(value).await?;
            }
        }

        Ok(())
    }
}

