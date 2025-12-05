use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;

pub struct RouterNode {
    pub key: String,
    pub value: Value,
    pub operator: String,
}

impl RouterNode {
    pub fn new(key: String, value: Value, operator: String) -> Self {
        Self { key, value, operator }
    }

    fn compare(&self, actual: &Value, expected: &Value) -> bool {
        match self.operator.as_str() {
            "==" => actual == expected || crate::stream_engine::utils::loose_eq(actual, expected),
            "!=" => actual != expected && !crate::stream_engine::utils::loose_eq(actual, expected),
            ">" => self.compare_numbers(actual, expected, |a, b| a > b),
            "<" => self.compare_numbers(actual, expected, |a, b| a < b),
            ">=" => self.compare_numbers(actual, expected, |a, b| a >= b),
            "<=" => self.compare_numbers(actual, expected, |a, b| a <= b),
            "contains" => self.check_contains(actual, expected),
            _ => actual == expected,
        }
    }

    fn compare_numbers<F>(&self, a: &Value, b: &Value, op: F) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        use crate::stream_engine::utils::to_f64;
        if let (Some(a_num), Some(b_num)) = (to_f64(a), to_f64(b)) {
             op(a_num, b_num)
        } else {
             false
        }
    }



    fn check_contains(&self, container: &Value, item: &Value) -> bool {
        match container {
            Value::String(s) => {
                if let Value::String(sub) = item {
                    s.contains(sub)
                } else {
                    s.contains(&item.to_string())
                }
            }
            Value::Array(arr) => arr.contains(item),
            _ => false,
        }
    }
}

#[async_trait]
impl StreamNode for RouterNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(rx) = inputs.get_mut(0) {
            while let Some(data) = rx.recv().await {
                // Compile and search inside a block to ensure `expr` (which is !Send) 
                // is dropped before we await on tx.send()
                let match_found = {
                    let expr = jmespath::compile(&self.key).map_err(|e| anyhow::anyhow!("Invalid JMESPath key: {}", e))?;
                    match expr.search(&data) {
                        Ok(result) => {
                            let result_json = serde_json::to_value(&*result).unwrap_or(Value::Null);
                            println!("Router Debug: Key='{}', Op='{}', Result={:?}, Expected={:?}", self.key, self.operator, result_json, self.value);
                            self.compare(&result_json, &self.value)
                        },
                        Err(_) => false,
                    }
                };
                
                if match_found {
                    // Send to Port 0 (True)
                    if let Some(tx) = outputs.first() {
                        let _ = tx.send(data).await;
                    }
                } else {
                    // Send to Port 1 (False)
                    if let Some(tx) = outputs.get(1) {
                        let _ = tx.send(data).await;
                    }
                }
            }
        }
        Ok(())
    }
}
