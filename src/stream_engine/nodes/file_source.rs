use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::{Result, anyhow};
use std::fs::File;
use std::io::BufReader;

pub struct FileSource {
    path: String,
}

impl FileSource {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}

#[async_trait]
impl StreamNode for FileSource {
    async fn run(&self, _inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        let file = File::open(&self.path).map_err(|e| anyhow!("Failed to open file {}: {}", self.path, e))?;
        let reader = BufReader::new(file);

        // Parse the entire file as a Value
        // Note: For very large files, we should use a streaming parser (like serde_json::Deserializer::from_reader).
        // But for "mock data" (10k items), loading into memory is probably fine for now.
        // Let's try to be slightly smarter and support Array iteration.
        
        let data: Value = serde_json::from_reader(reader)?;

        if let Some(array) = data.as_array() {
            if let Some(tx) = outputs.get(0) {
                for item in array {
                    tx.send(item.clone()).await?;
                }
            }
        } else {
            // If not an array, emit the single object
            if let Some(tx) = outputs.get(0) {
                tx.send(data).await?;
            }
        }

        Ok(())
    }
}
