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
    Key(Vec<String>, Vec<String>), // (Left Keys, Right Keys)
}

pub struct JoinNode {
    pub join_type: JoinType,
    pub mode: JoinMode,
}

impl JoinNode {
    pub fn new(join_type: JoinType, mode: JoinMode) -> Self {
        Self { join_type, mode }
    }

    fn extract_composite_key(value: &Value, keys: &[String]) -> Option<String> {
        let mut parts = Vec::new();
        for key in keys {
            let val_str = value.get(key).and_then(|v| {
                match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                }
            });
            
            if let Some(s) = val_str {
                parts.push(s);
            } else {
                return None; // All keys must be present
            }
        }
        Some(parts.join("\0")) // Use null char as separator
    }
}

#[async_trait]
impl StreamNode for JoinNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        // Ensure we have at least 2 inputs
        if inputs.len() < 2 {
            return Ok(());
        }

        let output = outputs.first();
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
                    } else if self.mode == JoinMode::Inner && (l_opt.is_none() || r_opt.is_none()) {
                        break;
                    }
                }
            }
            JoinType::Key(left_keys, right_keys) => {
                // Use Vec<Value> to support 1:N and M:N joins
                let mut left_buffer: HashMap<String, Vec<Value>> = HashMap::new();
                let mut right_buffer: HashMap<String, Vec<Value>> = HashMap::new();
                
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
                                    if let Some(k_string) = Self::extract_composite_key(&l, left_keys) {
                                        // Add to buffer
                                        left_buffer.entry(k_string.clone()).or_default().push(l.clone());
                                        
                                        // Initialize matched status if new key
                                        left_matched.entry(k_string.clone()).or_insert(false);

                                        // Check against all existing right buffer items
                                        if let Some(r_items) = right_buffer.get(&k_string) {
                                            for r in r_items {
                                                let merged = json!({ "left": l, "right": r });
                                                output.send(merged).await?;
                                            }
                                            // Mark as matched
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
                                    if let Some(k_string) = Self::extract_composite_key(&r, right_keys) {
                                        // Add to buffer
                                        right_buffer.entry(k_string.clone()).or_default().push(r.clone());
                                        
                                        // Initialize matched status if new key
                                        right_matched.entry(k_string.clone()).or_insert(false);

                                        // Check against all existing left buffer items
                                        if let Some(l_items) = left_buffer.get(&k_string) {
                                            for l in l_items {
                                                let merged = json!({ "left": l, "right": r });
                                                output.send(merged).await?;
                                            }
                                            // Mark as matched
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
                    for (k, l_items) in &left_buffer {
                        if !left_matched.get(k).copied().unwrap_or(false) {
                            // Emit each unmatched left item
                            for l in l_items {
                                let merged = json!({ "left": l, "right": Value::Null });
                                output.send(merged).await?;
                            }
                        }
                    }
                }

                if self.mode == JoinMode::Right || self.mode == JoinMode::Outer {
                    for (k, r_items) in &right_buffer {
                        if !right_matched.get(k).copied().unwrap_or(false) {
                            // Emit each unmatched right item
                            for r in r_items {
                                let merged = json!({ "left": Value::Null, "right": r });
                                output.send(merged).await?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}




