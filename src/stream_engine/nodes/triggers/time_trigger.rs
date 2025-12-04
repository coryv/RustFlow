use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{json, Value};
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{interval, Duration};

pub struct TimeTrigger {
    cron_expression: String, // Kept for metadata, though we just loop for now
    interval_seconds: u64,
}

impl TimeTrigger {
    pub fn new(cron_expression: String, interval_seconds: u64) -> Self {
        Self { cron_expression, interval_seconds }
    }
}

#[async_trait]
impl StreamNode for TimeTrigger {
    async fn run(&self, _inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(tx) = outputs.first() {
            let mut ticker = interval(Duration::from_secs(self.interval_seconds));

            // Loop forever (or until channel closed)
            loop {
                ticker.tick().await; // First tick completes immediately

                let start = SystemTime::now();
                let since_the_epoch = start
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");

                let data = json!({
                    "timestamp": since_the_epoch.as_secs(),
                    "cron": self.cron_expression
                });

                if tx.send(data).await.is_err() {
                    break; // Downstream closed
                }
            }
        }
        Ok(())
    }
}
