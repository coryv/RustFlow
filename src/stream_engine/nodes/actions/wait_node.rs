use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{json, Value};
use anyhow::Result;
use tokio::time::{timeout, Duration};

pub struct WaitNode {
    timeout_ms: Option<u64>,
}

impl WaitNode {
    pub fn new(timeout_ms: Option<u64>) -> Self {
        Self { timeout_ms }
    }
}

#[async_trait]
impl StreamNode for WaitNode {
    async fn run(&self, inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        use tokio_stream::StreamExt;
        use tokio_stream::wrappers::ReceiverStream;
        use tokio_stream::StreamMap;

        if inputs.is_empty() {
             return Ok(());
        }

        let num_inputs = inputs.len();
        
        // We need to keep ownership of the ReceiverStreams to re-insert them.
        // Option<ReceiverStream> allows us to take them out and put them back.
        let mut streams: Vec<Option<ReceiverStream<Value>>> = inputs
            .into_iter()
            .map(|rx| Some(ReceiverStream::new(rx)))
            .collect();

        // The active map we poll from. 
        // We only put streams here that we are currently waiting for data from.
        let mut active_map = StreamMap::new();

        // Initial population: wait for everyone
        for (i, stream_opt) in streams.iter_mut().enumerate() {
            if let Some(stream) = stream_opt.take() {
                active_map.insert(i, stream);
            }
        }

        // Store one item per input. 
        // None = waiting for data. Some = has data.
        let mut pending_values: Vec<Option<Value>> = vec![None; num_inputs];
        // Track streams that have permanently ended
        let mut closed_streams = vec![false; num_inputs];

        loop {
            // Check if we have gathered all inputs
            let all_present = pending_values.iter().enumerate().all(|(i, val)| {
                 val.is_some() || closed_streams[i]
            });

            // Also check if we truly have *active* data to emit (not just all closed)
            // If all are closed and we have partial data, we can't complete a full set, so we finish.
            // If all are closed and we have NO data, we finish.
            let all_closed = closed_streams.iter().all(|&c| c);

            if all_present && !all_closed {
                // If we have some data and others closed, we can't proceed with a "Wait" logic generally,
                // unless we treat closed as "no more events ever".
                // Standard Zip: If one ends, the whole thing ends because we can't make a pair.
                // So if any stream is closed and we don't have a value for it, we can't complete the batch.
                if closed_streams.iter().zip(pending_values.iter()).any(|(closed, val)| *closed && val.is_none()) {
                    break; 
                }

                // Emit!
                for (i, val_opt) in pending_values.iter_mut().enumerate() {
                    if let Some(val) = val_opt.take() {
                        if let Some(tx) = outputs.get(i) {
                            let _ = tx.send(val).await;
                        }
                    }
                }

                // Re-arm: Put streams back into the map to fetch the next batch
                for (i, stream_opt) in streams.iter_mut().enumerate() {
                     if let Some(stream) = stream_opt.take() {
                        active_map.insert(i, stream);
                    }
                }
                continue;
            } else if all_closed {
                 break;
            }

            // Wait for next item
            let next_item = if let Some(ms) = self.timeout_ms {
                match timeout(Duration::from_millis(ms), active_map.next()).await {
                    Ok(res) => res,
                    Err(_) => return Err(anyhow::anyhow!("WaitNode timed out")),
                }
            } else {
                active_map.next().await
            };

            match next_item {
                Some((idx, val)) => {
                    // We got data from stream `idx`
                    pending_values[idx] = Some(val);
                    
                    // CRITICAL FOR BACKPRESSURE:
                    // Remove this stream from the polling map so we don't read the NEXT item 
                    // until we are ready for the next batch.
                    if let Some(stream) = active_map.remove(&idx) {
                        streams[idx] = Some(stream);
                    }
                }
                None => {
                    // StreamMap returns None if empty.
                    // Or if a key is removed?
                    // If active_map is empty, it means we are waiting for nothing?
                    // That implies we are ready to emit (handled at top of loop).
                    // Or we are deadlocked?
                    break; 
                }
            }
        }

        Ok(())
    }
}
