use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;

pub enum UnionMode {
    Interleaved,
    Sequential,
}

pub struct UnionNode {
    mode: UnionMode,
}

impl UnionNode {
    pub fn new(mode: UnionMode) -> Self {
        Self { mode }
    }
}

#[async_trait]
impl StreamNode for UnionNode {
    async fn run(&self, inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(tx) = outputs.get(0) {
            match self.mode {
                UnionMode::Interleaved => {
                    // Use tokio::select! in a loop over all inputs?
                    // Or spawn a task for each input that forwards to the output?
                    // Spawning tasks is easiest for interleaving.
                    // But we need to keep the output channel open until ALL inputs are done.
                    // We can clone the sender for each task. When all tasks drop the sender, it closes?
                    // No, `tx` is kept alive by this function scope.
                    // We need to wait for all tasks to finish.
                    
                    let mut set = tokio::task::JoinSet::new();
                    
                    for mut rx in inputs {
                        let tx_clone = tx.clone();
                        set.spawn(async move {
                            while let Some(val) = rx.recv().await {
                                let _ = tx_clone.send(val).await;
                            }
                        });
                    }

                    while let Some(res) = set.join_next().await {
                        res?;
                    }
                }
                UnionMode::Sequential => {
                    // Process inputs one by one in order
                    for mut rx in inputs {
                        while let Some(val) = rx.recv().await {
                            tx.send(val).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
