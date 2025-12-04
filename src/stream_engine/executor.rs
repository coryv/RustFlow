use std::collections::HashMap;
use tokio::sync::{mpsc, broadcast};
use tokio::task::JoinSet;
use crate::stream_engine::{StreamNode, DebugConfig};
use crate::schema::ExecutionEvent;
use anyhow::{Result, anyhow, Context};
use serde_json::Value;

type InputsMap = HashMap<String, HashMap<usize, Vec<mpsc::Receiver<Value>>>>;
type OutputsMap = HashMap<String, HashMap<usize, Vec<mpsc::Sender<Value>>>>;

pub struct StreamExecutor {
    nodes: HashMap<String, Box<dyn StreamNode>>,
    // Edge: (from_id, from_port, to_id, to_port)
    edges: Vec<(String, usize, String, usize)>,
    event_sender: Option<broadcast::Sender<ExecutionEvent>>,
    debug_config: DebugConfig,
}

impl Default for StreamExecutor {
    fn default() -> Self {
        Self::new(DebugConfig::default())
    }
}

impl StreamExecutor {
    pub fn new(debug_config: DebugConfig) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            event_sender: None,
            debug_config,
        }
    }

    pub fn set_event_sender(&mut self, sender: broadcast::Sender<ExecutionEvent>) {
        self.event_sender = Some(sender);
    }

    pub fn add_node(&mut self, id: String, node: Box<dyn StreamNode>) {
        self.nodes.insert(id, node);
    }

    pub fn add_connection(&mut self, from: String, from_port: usize, to: String, to_port: usize) {
        self.edges.push((from, from_port, to, to_port));
    }

    pub async fn run(self) -> Result<()> {
        let (mut inputs, mut outputs) = self.initialize_channels()?;
        
        let mut set = JoinSet::new();

        for (id, node) in self.nodes {
            let node_inputs = Self::prepare_node_inputs(&id, &mut inputs)?;
            let node_outputs = Self::prepare_node_outputs(&id, &mut outputs);

            let event_sender = self.event_sender.clone();
            let node_id = id.clone();

            set.spawn(async move {
                // Emit NodeStart
                if let Some(sender) = &event_sender {
                    let _ = sender.send(ExecutionEvent::NodeStart { node_id: node_id.clone() });
                }

                let result = node.run(node_inputs, node_outputs).await;

                // Emit NodeFinish or NodeError
                if let Some(sender) = &event_sender {
                    match &result {
                        Ok(_) => {
                            let _ = sender.send(ExecutionEvent::NodeFinish { node_id });
                        }
                        Err(e) => {
                            let _ = sender.send(ExecutionEvent::NodeError { 
                                node_id, 
                                error: e.to_string() 
                            });
                        }
                    }
                }
                
                result
            });
        }

        while let Some(res) = set.join_next().await {
            res.context("Task join error")??;
        }

        Ok(())
    }

    fn initialize_channels(&self) -> Result<(InputsMap, OutputsMap)> {
        let mut inputs: InputsMap = HashMap::new();
        let mut outputs: OutputsMap = HashMap::new();

        // Initialize maps
        for id in self.nodes.keys() {
            inputs.insert(id.clone(), HashMap::new());
            outputs.insert(id.clone(), HashMap::new());
        }

        // Create channels for edges
        for (from, from_port, to, to_port) in &self.edges {
            let (tx, mut rx) = mpsc::channel::<Value>(100);
            let (tap_tx, tap_rx) = mpsc::channel::<Value>(100);
            
            // Spawn Tap Task
            let event_sender = self.event_sender.clone();
            let from_id = from.clone();
            let to_id = to.clone();
            let limit = self.debug_config.limit_records;
            
            tokio::spawn(async move {
                let mut count = 0;
                while let Some(val) = rx.recv().await {
                    // Broadcast event
                    if let Some(sender) = &event_sender {
                        let _ = sender.send(ExecutionEvent::EdgeData {
                            from: from_id.clone(),
                            to: to_id.clone(),
                            value: val.clone(),
                        });
                    }
                    
                    // Forward to destination with limit check
                    if let Some(limit_val) = limit {
                        if count < limit_val {
                            if tap_tx.send(val).await.is_err() {
                                break;
                            }
                            count += 1;
                        } else {
                            // Drop excess records (drain)
                            // We continue loop to keep upstream channel open
                            continue; 
                        }
                    } else {
                        // No limit
                        if tap_tx.send(val).await.is_err() {
                            break;
                        }
                    }
                }
            });

            // Add tap_rx to destination
            if let Some(node_inputs) = inputs.get_mut(to) {
                node_inputs.entry(*to_port).or_default().push(tap_rx);
            } else {
                return Err(anyhow!("Edge points to unknown node: {}", to));
            }

            // Add tx to source
            if let Some(node_outputs) = outputs.get_mut(from) {
                node_outputs.entry(*from_port).or_default().push(tx);
            } else {
                 return Err(anyhow!("Edge starts from unknown node: {}", from));
            }
        }

        Ok((inputs, outputs))
    }

    fn prepare_node_inputs(id: &str, inputs: &mut InputsMap) -> Result<Vec<mpsc::Receiver<Value>>> {
        let mut node_input_map = inputs.remove(id).ok_or_else(|| anyhow!("Node inputs not found for {}", id))?;
        
        if node_input_map.is_empty() {
            return Ok(Vec::new());
        }

        let max_input_port = node_input_map.keys().max().copied().unwrap_or(0);
        let mut node_inputs_vec = Vec::new();
        
        for i in 0..=max_input_port {
            if let Some(mut rxs) = node_input_map.remove(&i) {
                if rxs.is_empty() {
                    let (_tx, rx) = mpsc::channel(1);
                    node_inputs_vec.push(rx);
                } else if rxs.len() == 1 {
                    node_inputs_vec.push(rxs.pop().unwrap());
                } else {
                    // Merge multiple inputs to one receiver
                    let (tx, rx) = mpsc::channel(100);
                    for mut r in rxs {
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            while let Some(val) = r.recv().await {
                                let _ = tx_clone.send(val).await;
                            }
                        });
                    }
                    node_inputs_vec.push(rx);
                }
            } else {
                // No input for this port, provide dummy closed receiver
                let (_tx, rx) = mpsc::channel(1);
                node_inputs_vec.push(rx);
            }
        }
        Ok(node_inputs_vec)
    }

    fn prepare_node_outputs(id: &str, outputs: &mut OutputsMap) -> Vec<mpsc::Sender<Value>> {
        let mut node_output_map = outputs.remove(id).unwrap_or_default();
        let max_output_port = node_output_map.keys().max().copied().unwrap_or(0);
        let mut node_outputs_vec = Vec::new();

        for i in 0..=max_output_port {
            let txs = node_output_map.remove(&i).unwrap_or_default();
            
            // We need to provide ONE sender to the node for this port.
            // If there are multiple downstream edges (Fan-Out), we broadcast.
            let (internal_tx, mut internal_rx) = mpsc::channel::<Value>(100);
            
            if !txs.is_empty() {
                tokio::spawn(async move {
                    while let Some(val) = internal_rx.recv().await {
                        for tx in &txs {
                            let _ = tx.send(val.clone()).await;
                        }
                    }
                });
            } else {
                 tokio::spawn(async move {
                    while (internal_rx.recv().await).is_some() {}
                });
            }
            node_outputs_vec.push(internal_tx);
        }
        node_outputs_vec
    }
}
