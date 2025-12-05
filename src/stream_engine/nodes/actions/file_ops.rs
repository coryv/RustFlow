use crate::stream_engine::StreamNode;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader, AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct FileReadNode {
    path: String,
    stream_lines: bool,
}

impl FileReadNode {
    pub fn new(config: Value) -> Self {
        let path = config.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let stream_lines = config.get("stream_lines").and_then(|v| v.as_bool()).unwrap_or(false);
        Self { path, stream_lines }
    }
}

#[async_trait]
impl StreamNode for FileReadNode {
    async fn run(
        &self,
        mut inputs: Vec<mpsc::Receiver<Value>>,
        outputs: Vec<mpsc::Sender<Value>>,
    ) -> anyhow::Result<()> {
        let mut rx = if !inputs.is_empty() {
             inputs.remove(0)
        } else {
             // If manual trigger or start node? Usually needs input trigger.
             // If trigger, we run once. If Action, we wait for input.
             // Let's assume Action behavior: wait for input signal.
             return Ok(());
        };

        let tx = if !outputs.is_empty() { outputs[0].clone() } else { return Ok(()); };

        while let Some(input) = rx.recv().await {
            // Path can be templated? For now static from config, or we could render it.
            // Let's assume static path for simplicity as per struct, 
            // but advanced usage requires dynamic path.
            // Let's handle dynamic path from config if it contains {{ }} later.
            
            let file = fs::File::open(&self.path).await?;
            
            if self.stream_lines {
                let reader = BufReader::new(file);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let output = serde_json::json!({
                        "content": line,
                        "path": self.path,
                        "original_input": input
                    });
                     if tx.send(output).await.is_err() {
                        return Ok(());
                    }
                }
            } else {
                let content = fs::read_to_string(&self.path).await?;
                let output = serde_json::json!({
                    "content": content,
                    "path": self.path,
                    "original_input": input
                });
                if tx.send(output).await.is_err() {
                     return Ok(());
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct FileWriteNode {
    path: String,
    content_template: String,
    mode: String, // "overwrite" | "append"
}

impl FileWriteNode {
    pub fn new(config: Value) -> Self {
        let path = config.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let content_template = config.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let mode = config.get("mode").and_then(|v| v.as_str()).unwrap_or("overwrite").to_string();
        Self { path, content_template, mode }
    }
}

#[async_trait]
impl StreamNode for FileWriteNode {
    async fn run(
        &self,
        mut inputs: Vec<mpsc::Receiver<Value>>,
        outputs: Vec<mpsc::Sender<Value>>,
    ) -> anyhow::Result<()> {
        if inputs.is_empty() { return Ok(()); }
        let mut rx = inputs.remove(0);
        let tx = if !outputs.is_empty() { Some(outputs[0].clone()) } else { None };

        while let Some(input) = rx.recv().await {
            // Render content
             let env = crate::stream_engine::expressions::create_environment();
             let content = match env.render_str(&self.content_template, &input) {
                 Ok(s) => s,
                 Err(e) => {
                     eprintln!("FileWriteNode Template Error: {}", e);
                     continue;
                 }
             };

             let mut file = if self.mode == "append" {
                 fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(&self.path)
                    .await?
             } else {
                 fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(&self.path)
                    .await?
             };

             file.write_all(content.as_bytes()).await?;
             // Ensure newline if appending? Optional. User controls content.
             
             if let Some(sender) = &tx {
                 sender.send(input).await.ok();
             }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ListDirNode {
    path: String,
    recursive: bool,
    pattern: String,
}

impl ListDirNode {
    pub fn new(config: Value) -> Self {
        let path = config.get("path").and_then(|v| v.as_str()).unwrap_or(".").to_string();
        let recursive = config.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);
        let pattern = config.get("pattern").and_then(|v| v.as_str()).unwrap_or("*").to_string();
        Self { path, recursive, pattern }
    }
}

#[async_trait]
impl StreamNode for ListDirNode {
    async fn run(
        &self,
        mut inputs: Vec<mpsc::Receiver<Value>>,
        outputs: Vec<mpsc::Sender<Value>>,
    ) -> anyhow::Result<()> {
        if inputs.is_empty() { return Ok(()); }
        let mut rx = inputs.remove(0);
        let tx = if !outputs.is_empty() { outputs[0].clone() } else { return Ok(()); };

        while let Some(input) = rx.recv().await {
             let mut entries = Vec::new();
             // Simple recursive walker
             let mut dirs = vec![std::path::PathBuf::from(&self.path)];
             
             while let Some(dir) = dirs.pop() {
                 let mut read_dir = match fs::read_dir(&dir).await {
                     Ok(rd) => rd,
                     Err(e) => {
                         eprintln!("Failed to read dir {:?}: {}", dir, e);
                         continue;
                     }
                 };

                 while let Ok(Some(entry)) = read_dir.next_entry().await {
                     let path = entry.path();
                     if path.is_dir() {
                         if self.recursive {
                             dirs.push(path.clone());
                         }
                     } else {
                         // File check
                         if matches_pattern(&path, &self.pattern) {
                             entries.push(json_file_entry(&path).await);
                         }
                     }
                 }
             }

             // Emit LIST of files, or stream them? 
             // Usually "List" node emits an Array. 
             // If we want streaming, we'd loop and send individually.
             // Let's emit Array for now as per established patterns (splitting can happen next).
             
             let output = serde_json::json!({
                 "files": entries,
                 "count": entries.len(),
                 "original_input": input
             });
             
             if tx.send(output).await.is_err() {
                 return Ok(());
             }
        }
        Ok(())
    }
}

fn matches_pattern(path: &std::path::Path, pattern: &str) -> bool {
    if pattern == "*" { return true; }
    // Simple extension match e.g. "*.csv"
    if pattern.starts_with("*.") {
        if let Some(ext) = path.extension() {
            return ext.to_string_lossy() == &pattern[2..];
        }
    }
    // Simple name match
    if let Some(name) = path.file_name() {
        return name.to_string_lossy() == pattern;
    }
    false
}

async fn json_file_entry(path: &std::path::Path) -> Value {
    let metadata = fs::metadata(path).await.ok();
    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified = metadata.as_ref().and_then(|m| m.modified().ok()).map(|t| {
        chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()
    }).unwrap_or_default();

    serde_json::json!({
        "path": path.to_string_lossy(),
        "name": path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(),
        "size": size,
        "modified": modified
    })
}
