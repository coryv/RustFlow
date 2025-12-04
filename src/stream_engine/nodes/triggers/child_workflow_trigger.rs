use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{json, Value};
use anyhow::Result;

pub struct ChildWorkflowTrigger;

#[async_trait]
impl StreamNode for ChildWorkflowTrigger {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(tx) = outputs.first() {
            if let Some(rx) = inputs.get_mut(0) {
                // If we have an input channel (injected data), read from it
                let mut received_any = false;
                while let Some(data) = rx.recv().await {
                    tx.send(data).await?;
                    received_any = true;
                }
                
                if !received_any {
                    // If no data was injected, we might want to error or just do nothing?
                    // For a child workflow, it usually expects input.
                    // But maybe it can run without input?
                    // Let's send null if nothing received, similar to ManualTrigger, 
                    // but maybe log a warning?
                }
            } else {
                // No inputs provided at all.
                // This happens if the executor didn't inject anything.
                // We should probably send null to start the flow.
                tx.send(json!(null)).await?;
            }
        }
        Ok(())
    }
}
