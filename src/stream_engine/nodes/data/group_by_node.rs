use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{json, Value};
use anyhow::Result;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregation {
    pub column: String,
    pub function: String, // sum, count, avg, min, max
    pub alias: Option<String>,
}

pub struct GroupByNode {
    pub group_by: Vec<String>,
    pub aggregations: Vec<Aggregation>,
}

impl GroupByNode {
    pub fn new(group_by: Vec<String>, aggregations: Vec<Aggregation>) -> Self {
        Self { group_by, aggregations }
    }

    fn extract_group_key(value: &Value, keys: &[String]) -> String {
        let mut parts = Vec::new();
        for key in keys {
            let val_str = value.get(key).map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
            parts.push(val_str);
        }
        parts.join("\0")
    }
}

#[async_trait]
impl StreamNode for GroupByNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if inputs.is_empty() || outputs.is_empty() {
            return Ok(());
        }

        let mut input = inputs.remove(0);
        let output = outputs.get(0).unwrap();

        let mut groups: HashMap<String, Vec<Value>> = HashMap::new();

        // 1. Collect all data
        while let Some(value) = input.recv().await {
            let key = Self::extract_group_key(&value, &self.group_by);
            groups.entry(key).or_default().push(value);
        }

        // 2. Aggregate and emit
        for (_key, records) in groups {
            if records.is_empty() {
                continue;
            }

            let mut result_obj = serde_json::Map::new();

            // Add group keys from the first record
            let first_record = &records[0];
            for key in &self.group_by {
                if let Some(val) = first_record.get(key) {
                    result_obj.insert(key.clone(), val.clone());
                } else {
                     result_obj.insert(key.clone(), Value::Null);
                }
            }

            // Perform aggregations
            for agg in &self.aggregations {
                let target_key = agg.alias.as_ref().unwrap_or(&agg.column);
                
                let val = match agg.function.as_str() {
                    "count" => json!(records.len()),
                    "sum" => {
                        let sum: f64 = records.iter()
                            .filter_map(|r| r.get(&agg.column).and_then(|v| v.as_f64()))
                            .sum();
                        json!(sum)
                    },
                    "avg" => {
                        let count = records.len() as f64;
                        let sum: f64 = records.iter()
                            .filter_map(|r| r.get(&agg.column).and_then(|v| v.as_f64()))
                            .sum();
                        if count > 0.0 {
                            json!(sum / count)
                        } else {
                            json!(0.0)
                        }
                    },
                    "min" => {
                        let min = records.iter()
                            .filter_map(|r| r.get(&agg.column).and_then(|v| v.as_f64()))
                            .fold(f64::INFINITY, f64::min);
                         if min == f64::INFINITY { Value::Null } else { json!(min) }
                    },
                    "max" => {
                        let max = records.iter()
                            .filter_map(|r| r.get(&agg.column).and_then(|v| v.as_f64()))
                            .fold(f64::NEG_INFINITY, f64::max);
                        if max == f64::NEG_INFINITY { Value::Null } else { json!(max) }
                    },
                    "median" => {
                        let mut values: Vec<f64> = records.iter()
                            .filter_map(|r| r.get(&agg.column).and_then(|v| v.as_f64()))
                            .collect();
                        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        let len = values.len();
                        if len == 0 {
                            Value::Null
                        } else if len % 2 == 1 {
                            json!(values[len / 2])
                        } else {
                            let mid = len / 2;
                            json!((values[mid - 1] + values[mid]) / 2.0)
                        }
                    },
                    "variance" => {
                        let values: Vec<f64> = records.iter()
                            .filter_map(|r| r.get(&agg.column).and_then(|v| v.as_f64()))
                            .collect();
                        let len = values.len();
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
                        let values: Vec<f64> = records.iter()
                            .filter_map(|r| r.get(&agg.column).and_then(|v| v.as_f64()))
                            .collect();
                        let len = values.len();
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
                
                result_obj.insert(target_key.clone(), val);
            }

            output.send(Value::Object(result_obj)).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_group_by_node() {
        let (tx, rx) = mpsc::channel(10);
        let (out_tx, mut out_rx) = mpsc::channel(10);

        let node = GroupByNode::new(
            vec!["category".to_string()],
            vec![
                Aggregation { column: "value".to_string(), function: "sum".to_string(), alias: Some("total_value".to_string()) },
                Aggregation { column: "value".to_string(), function: "count".to_string(), alias: Some("count".to_string()) },
            ]
        );

        tokio::spawn(async move {
            let data = vec![
                json!({"category": "A", "value": 10}),
                json!({"category": "B", "value": 20}),
                json!({"category": "A", "value": 5}),
                json!({"category": "B", "value": 15}),
                json!({"category": "C", "value": 100}),
            ];
            for d in data {
                tx.send(d).await.unwrap();
            }
        });

        node.run(vec![rx], vec![out_tx]).await.unwrap();

        let mut results = Vec::new();
        while let Some(val) = out_rx.recv().await {
            results.push(val);
        }

        // Sort results by category for deterministic assertion
        results.sort_by(|a, b| a["category"].as_str().unwrap().cmp(b["category"].as_str().unwrap()));

        assert_eq!(results.len(), 3);
        
        // A: 10 + 5 = 15, count 2
        assert_eq!(results[0]["category"], "A");
        assert_eq!(results[0]["total_value"], 15.0);
        assert_eq!(results[0]["count"], 2);

        // B: 20 + 15 = 35, count 2
        assert_eq!(results[1]["category"], "B");
        assert_eq!(results[1]["total_value"], 35.0);
        assert_eq!(results[1]["count"], 2);

        // C: 100, count 1
        assert_eq!(results[2]["category"], "C");
        assert_eq!(results[2]["total_value"], 100.0);
        assert_eq!(results[2]["count"], 1);
    }

    #[tokio::test]
    async fn test_group_by_node_stats() {
        let (tx, rx) = mpsc::channel(10);
        let (out_tx, mut out_rx) = mpsc::channel(10);

        let node = GroupByNode::new(
            vec!["category".to_string()],
            vec![
                Aggregation { column: "value".to_string(), function: "median".to_string(), alias: Some("median".to_string()) },
                Aggregation { column: "value".to_string(), function: "variance".to_string(), alias: Some("variance".to_string()) },
                Aggregation { column: "value".to_string(), function: "stddev".to_string(), alias: Some("stddev".to_string()) },
            ]
        );

        tokio::spawn(async move {
            let data = vec![
                // Group A: 1, 2, 3, 4, 5. Median=3, Mean=3, Var=2.5, StdDev=~1.58
                json!({"category": "A", "value": 1}),
                json!({"category": "A", "value": 2}),
                json!({"category": "A", "value": 3}),
                json!({"category": "A", "value": 4}),
                json!({"category": "A", "value": 5}),
            ];
            for d in data {
                tx.send(d).await.unwrap();
            }
        });

        node.run(vec![rx], vec![out_tx]).await.unwrap();

        let val = out_rx.recv().await.unwrap();
        assert_eq!(val["category"], "A");
        assert_eq!(val["median"], 3.0);
        assert_eq!(val["variance"], 2.5);
        let stddev = val["stddev"].as_f64().unwrap();
        assert!((stddev - 1.58113883).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_group_by_multi_column() {
        let (tx, rx) = mpsc::channel(10);
        let (out_tx, mut out_rx) = mpsc::channel(10);

        let node = GroupByNode::new(
            vec!["region".to_string(), "category".to_string()],
            vec![
                Aggregation { column: "value".to_string(), function: "sum".to_string(), alias: Some("total".to_string()) },
            ]
        );

        tokio::spawn(async move {
            let data = vec![
                json!({"region": "US", "category": "A", "value": 10}),
                json!({"region": "US", "category": "A", "value": 20}), // US-A: 30
                json!({"region": "US", "category": "B", "value": 5}),  // US-B: 5
                json!({"region": "EU", "category": "A", "value": 15}), // EU-A: 15
            ];
            for d in data {
                tx.send(d).await.unwrap();
            }
        });

        node.run(vec![rx], vec![out_tx]).await.unwrap();

        let mut results = Vec::new();
        while let Some(val) = out_rx.recv().await {
            results.push(val);
        }

        // Sort by region then category
        results.sort_by(|a, b| {
            let r = a["region"].as_str().unwrap().cmp(b["region"].as_str().unwrap());
            if r == std::cmp::Ordering::Equal {
                a["category"].as_str().unwrap().cmp(b["category"].as_str().unwrap())
            } else {
                r
            }
        });

        assert_eq!(results.len(), 3);

        // EU-A
        assert_eq!(results[0]["region"], "EU");
        assert_eq!(results[0]["category"], "A");
        assert_eq!(results[0]["total"], 15.0);

        // US-A
        assert_eq!(results[1]["region"], "US");
        assert_eq!(results[1]["category"], "A");
        assert_eq!(results[1]["total"], 30.0);

        // US-B
        assert_eq!(results[2]["region"], "US");
        assert_eq!(results[2]["category"], "B");
        assert_eq!(results[2]["total"], 5.0);
    }
}
