use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{json, Value};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsNode {
    pub columns: Vec<String>,
    pub operations: Vec<String>, // mean, median, variance, stddev, min, max, sum, count
}

impl StatsNode {
    pub fn new(columns: Vec<String>, operations: Vec<String>) -> Self {
        Self { columns, operations }
    }
}

#[async_trait]
impl StreamNode for StatsNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if inputs.is_empty() || outputs.is_empty() {
            return Ok(());
        }

        let mut input = inputs.remove(0);
        let output = outputs.first().unwrap();

        let mut data_map: HashMap<String, Vec<f64>> = HashMap::new();
        for col in &self.columns {
            data_map.insert(col.clone(), Vec::new());
        }

        // 1. Collect all data
        while let Some(value) = input.recv().await {
            for col in &self.columns {
                if let Some(val) = value.get(col).and_then(|v| v.as_f64()) {
                    data_map.entry(col.clone()).or_default().push(val);
                }
            }
        }

        // 2. Calculate stats
        let mut result_obj = serde_json::Map::new();

        for col in &self.columns {
            let values = data_map.get_mut(col).unwrap();
            let len = values.len();
            let mut col_stats = serde_json::Map::new();

            if len == 0 {
                result_obj.insert(col.clone(), Value::Null);
                continue;
            }

            for op in &self.operations {
                let val = match op.as_str() {
                    "count" => json!(len),
                    "sum" => json!(values.iter().sum::<f64>()),
                    "mean" | "avg" => {
                        let sum: f64 = values.iter().sum();
                        json!(sum / len as f64)
                    },
                    "min" => {
                        let min = values.iter().fold(f64::INFINITY, |a, &b| f64::min(a, b));
                        json!(min)
                    },
                    "max" => {
                        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| f64::max(a, b));
                        json!(max)
                    },
                    "median" => {
                        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        if len % 2 == 1 {
                            json!(values[len / 2])
                        } else {
                            let mid = len / 2;
                            json!((values[mid - 1] + values[mid]) / 2.0)
                        }
                    },
                    "variance" => {
                        if len < 2 {
                            json!(0.0)
                        } else {
                            let sum: f64 = values.iter().sum();
                            let mean = sum / len as f64;
                            let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (len - 1) as f64;
                            json!(variance)
                        }
                    },
                    "stddev" => {
                        if len < 2 {
                            json!(0.0)
                        } else {
                            let sum: f64 = values.iter().sum();
                            let mean = sum / len as f64;
                            let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (len - 1) as f64;
                            json!(variance.sqrt())
                        }
                    },
                    _ => Value::Null,
                };
                col_stats.insert(op.clone(), val);
            }
            result_obj.insert(col.clone(), Value::Object(col_stats));
        }

        output.send(Value::Object(result_obj)).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_stats_node() {
        let (tx, rx) = mpsc::channel(10);
        let (out_tx, mut out_rx) = mpsc::channel(10);

        let node = StatsNode::new(
            vec!["score".to_string(), "age".to_string()],
            vec!["mean".to_string(), "max".to_string(), "count".to_string()]
        );

        tokio::spawn(async move {
            let data = vec![
                json!({"score": 10, "age": 20}),
                json!({"score": 20, "age": 30}),
                json!({"score": 30, "age": 40}),
            ];
            for d in data {
                tx.send(d).await.unwrap();
            }
        });

        node.run(vec![rx], vec![out_tx]).await.unwrap();

        let result = out_rx.recv().await.unwrap();
        
        // Score: 10, 20, 30 -> Mean 20, Max 30, Count 3
        let score_stats = result.get("score").unwrap();
        assert_eq!(score_stats.get("mean").unwrap(), 20.0);
        assert_eq!(score_stats.get("max").unwrap(), 30.0);
        assert_eq!(score_stats.get("count").unwrap(), 3);

        // Age: 20, 30, 40 -> Mean 30, Max 40, Count 3
        let age_stats = result.get("age").unwrap();
        assert_eq!(age_stats.get("mean").unwrap(), 30.0);
        assert_eq!(age_stats.get("max").unwrap(), 40.0);
    }
}
