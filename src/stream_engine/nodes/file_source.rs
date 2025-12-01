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
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        let output = outputs.get(0).ok_or_else(|| anyhow!("FileSource has no output"))?;
        
        // If we have inputs, wait for a signal/path
        if !inputs.is_empty() {
            let mut rx = inputs.remove(0);
            while let Some(input) = rx.recv().await {
                // Determine path: Input string > Input object "path" > Config path
                let path = if let Some(s) = input.as_str() {
                    s.to_string()
                } else if let Some(s) = input.get("path").and_then(|v| v.as_str()) {
                    s.to_string()
                } else {
                    self.path.clone()
                };

                self.process_file(&path, output).await?;
            }
        } else {
            // No inputs, run once using config path (Trigger mode)
            self.process_file(&self.path, output).await?;
        }

        Ok(())
    }
}

impl FileSource {
    async fn process_file(&self, path: &str, tx: &Sender<Value>) -> Result<()> {
        let file = File::open(path).map_err(|e| anyhow!("Failed to open file {}: {}", path, e))?;
        let reader = BufReader::new(file);
        let data: Value = serde_json::from_reader(reader)?;

        if let Some(array) = data.as_array() {
            for item in array {
                tx.send(item.clone()).await?;
            }
        } else {
            tx.send(data).await?;
        }
        Ok(())
    }
}
