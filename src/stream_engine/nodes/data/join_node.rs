use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{json, Value};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoinMode {
    Inner,
    Left,
    Right,
    Outer,
}

pub enum JoinType {
    Index,
    Key(String, String), // (Left Key, Right Key)
}

pub struct JoinNode {
    pub join_type: JoinType,
    pub mode: JoinMode,
}

impl JoinNode {
    pub fn new(join_type: JoinType, mode: JoinMode) -> Self {
        Self { join_type, mode }
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
                let mut left = inputs.remove(0);
                let mut right = inputs.remove(0);

                loop {
                    let l_opt = left.recv().await;
                    let r_opt = right.recv().await;

                    if l_opt.is_none() && r_opt.is_none() {
                        break;
                    }

                    let should_emit = match self.mode {
                        JoinMode::Inner => l_opt.is_some() && r_opt.is_some(),
                        JoinMode::Left => l_opt.is_some(),
                        JoinMode::Right => r_opt.is_some(),
                        JoinMode::Outer => l_opt.is_some() || r_opt.is_some(),
                    };

                    if should_emit {
                        let merged = json!({
                            "left": l_opt.unwrap_or(Value::Null),
                            "right": r_opt.unwrap_or(Value::Null)
                        });
                        output.send(merged).await?;
                    } else {
                        // If we are in Inner mode and one is missing, we stop? 
                        // Actually for Index join, usually it stops when shortest stream ends for Inner.
                        // For Outer, it continues until longest ends.
                        // The above logic consumes one from each.
                        // If one stream ends early, recv() returns None forever.
                        // So the logic holds:
                        // Inner: stops as soon as one is None (because l_opt && r_opt will be false).
                        // Left: continues as long as Left has value. Right will be Null if exhausted.
                        // Right: continues as long as Right has value. Left will be Null if exhausted.
                        // Outer: continues until both are exhausted.
                        
                        // Optimization: Break early for Inner if one is done?
                        if self.mode == JoinMode::Inner && (l_opt.is_none() || r_opt.is_none()) {
                            break;
                        }
                    }
                }
            }
            JoinType::Key(left_key, right_key) => {
                let mut left_buffer: HashMap<String, Value> = HashMap::new();
                let mut right_buffer: HashMap<String, Value> = HashMap::new();
                
                // Track which keys have been matched to handle Left/Right/Outer logic correctly?
                // Actually, for Hash Join:
                // - Inner: emit on match.
                // - Left: emit on match. If no match by end, emit lefts that didn't match? 
                //   Stream join is tricky. Usually we buffer one side or both.
                //   If we assume infinite streams, we can't do full outer/left without windowing.
                //   Here we assume finite streams (batch-like) as per previous implementation.
                
                // We need to track if a buffered item was ever matched?
                // Or do we just emit immediately on match?
                // If we emit on match, we are doing an Inner join on the intersection.
                
                // For Left Join:
                // We need to know if a Left item *never* found a Right match.
                // We can only know this when the Right stream closes (or we have a window).
                // Since we are buffering everything, we can track "matched" status.
                
                let mut left_matched: HashMap<String, bool> = HashMap::new();
                let mut right_matched: HashMap<String, bool> = HashMap::new();

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
                                        let k_string = k_val.to_string();
                                        left_buffer.insert(k_string.clone(), l.clone());
                                        left_matched.insert(k_string.clone(), false);

                                        // Check against all existing right buffer items
                                        // Note: This logic assumes 1:1 or 1:N? 
                                        // Standard hash join checks existence.
                                        // If there are multiple right items with same key, we should emit multiple?
                                        // The current buffer is HashMap<String, Value>, so it only stores LAST value for a key.
                                        // This is a limitation of the current implementation. 
                                        // To support proper join, buffer should be HashMap<String, Vec<Value>>.
                                        // For now, I will stick to the existing limitation but add the mode logic.
                                        
                                        if let Some(r) = right_buffer.get(&k_string) {
                                            let merged = json!({ "left": l, "right": r });
                                            output.send(merged).await?;
                                            left_matched.insert(k_string.clone(), true);
                                            right_matched.insert(k_string.clone(), true);
                                        }
                                    }
                                }
                                None => left_closed = true,
                            }
                        }
                        r_opt = right.recv(), if !right_closed => {
                            match r_opt {
                                Some(r) => {
                                    if let Some(k_val) = r.get(right_key).and_then(|v| v.as_str()) {
                                        let k_string = k_val.to_string();
                                        right_buffer.insert(k_string.clone(), r.clone());
                                        right_matched.insert(k_string.clone(), false);

                                        if let Some(l) = left_buffer.get(&k_string) {
                                            let merged = json!({ "left": l, "right": r });
                                            output.send(merged).await?;
                                            right_matched.insert(k_string.clone(), true);
                                            left_matched.insert(k_string.clone(), true);
                                        }
                                    }
                                }
                                None => right_closed = true,
                            }
                        }
                    }
                }

                // Post-processing for non-Inner joins
                if self.mode == JoinMode::Left || self.mode == JoinMode::Outer {
                    for (k, l) in &left_buffer {
                        if !left_matched.get(k).copied().unwrap_or(false) {
                            let merged = json!({ "left": l, "right": Value::Null });
                            output.send(merged).await?;
                        }
                    }
                }

                if self.mode == JoinMode::Right || self.mode == JoinMode::Outer {
                    for (k, r) in &right_buffer {
                        if !right_matched.get(k).copied().unwrap_or(false) {
                            let merged = json!({ "left": Value::Null, "right": r });
                            output.send(merged).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
