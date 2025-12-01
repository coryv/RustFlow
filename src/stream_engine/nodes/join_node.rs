use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{json, Value};
use anyhow::Result;
use std::collections::HashMap;

pub enum JoinType {
    Index,
    Key(String, String), // (Left Key, Right Key)
}

pub struct JoinNode {
    pub join_type: JoinType,
}

impl JoinNode {
    pub fn new(join_type: JoinType) -> Self {
        Self { join_type }
    }
}

#[async_trait]
impl StreamNode for JoinNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        // Ensure we have at least 2 inputs
        if inputs.len() < 2 {
            return Ok(());
        }

        let output = outputs.get(0);
        if output.is_none() {
            return Ok(());
        }
        let output = output.unwrap();

        match &self.join_type {
            JoinType::Index => {
                // Zip inputs: Wait for one from Left and one from Right
                let mut left = inputs.remove(0);
                let mut right = inputs.remove(0);

                loop {
                    let l = left.recv().await;
                    let r = right.recv().await;

                    if l.is_none() || r.is_none() {
                        break;
                    }

                    let merged = json!({
                        "left": l.unwrap(),
                        "right": r.unwrap()
                    });
                    output.send(merged).await?;
                }
            }
            JoinType::Key(left_key, right_key) => {
                // Hash Join logic
                // We need to buffer inputs until we find a match.
                // For simplicity, let's buffer everything in memory (Hash Join).
                // Warning: Unbounded memory usage if keys don't match.
                
                let mut left_buffer: HashMap<String, Value> = HashMap::new();
                let mut right_buffer: HashMap<String, Value> = HashMap::new();
                
                // We need to listen to BOTH inputs concurrently.
                // We can't just select! because we need mutable access to buffers.
                // We'll use a loop with tokio::select! on the receivers.
                // But we have them in a Vec. We need to split them out.
                
                let (mut left, mut right) = (inputs.remove(0), inputs.remove(0));

                let mut left_closed = false;
                let mut right_closed = false;

                loop {
                    if left_closed && right_closed {
                        break;
                    }

                    tokio::select! {
                        l_opt = left.recv(), if !left_closed => {
                            match l_opt {
                                Some(l) => {
                                    if let Some(k_val) = l.get(left_key).and_then(|v| v.as_str()) {
                                        if let Some(r) = right_buffer.remove(k_val) {
                                            let merged = json!({ "left": l, "right": r });
                                            output.send(merged).await?;
                                        } else {
                                            left_buffer.insert(k_val.to_string(), l);
                                        }
                                    }
                                }
                                None => {
                                    left_closed = true;
                                }
                            }
                        }
                        r_opt = right.recv(), if !right_closed => {
                            match r_opt {
                                Some(r) => {
                                    if let Some(k_val) = r.get(right_key).and_then(|v| v.as_str()) {
                                        if let Some(l) = left_buffer.remove(k_val) {
                                            let merged = json!({ "left": l, "right": r });
                                            output.send(merged).await?;
                                        } else {
                                            right_buffer.insert(k_val.to_string(), r);
                                        }
                                    }
                                }
                                None => {
                                    right_closed = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
