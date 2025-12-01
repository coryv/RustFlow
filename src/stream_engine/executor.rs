use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use crate::stream_engine::StreamNode;
use anyhow::{Result, anyhow};
use serde_json::Value;

pub struct StreamExecutor {
    nodes: HashMap<String, Box<dyn StreamNode>>,
    // Edge: (from_id, from_port, to_id, to_port)
    edges: Vec<(String, usize, String, usize)>,
}

impl StreamExecutor {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, id: String, node: Box<dyn StreamNode>) {
        self.nodes.insert(id, node);
    }

    pub fn add_connection(&mut self, from: String, from_port: usize, to: String, to_port: usize) {
        self.edges.push((from, from_port, to, to_port));
    }

    pub async fn run(self) -> Result<()> {
        // NodeID -> PortIndex -> Receiver
        let mut inputs: HashMap<String, HashMap<usize, Vec<mpsc::Receiver<Value>>>> = HashMap::new();
        // NodeID -> PortIndex -> Senders
        let mut outputs: HashMap<String, HashMap<usize, Vec<mpsc::Sender<Value>>>> = HashMap::new();

        // Initialize maps
        for id in self.nodes.keys() {
            inputs.insert(id.clone(), HashMap::new());
            outputs.insert(id.clone(), HashMap::new());
        }

        // Create channels for edges
        for (from, from_port, to, to_port) in &self.edges {
            let (tx, rx) = mpsc::channel::<Value>(100);
            
            // Add rx to destination
            if let Some(node_inputs) = inputs.get_mut(to) {
                node_inputs.entry(*to_port).or_default().push(rx);
            } else {
                return Err(anyhow!("Edge points to unknown node: {}", to));
            }

            // Add tx to source
            if let Some(node_outputs) = outputs.get_mut(from) {
                node_outputs.entry(*from_port).or_default().push(tx);
            }
        }

        let mut set = JoinSet::new();

        for (id, node) in self.nodes {
            // Flatten inputs: We need a Vec<Receiver>. 
            // Assumption: Nodes expect inputs at specific indices.
            // We'll create a sparse vector based on the max port index.
            let mut node_input_map = inputs.remove(&id).unwrap();
            let max_input_port = node_input_map.keys().max().copied().unwrap_or(0);
            let mut node_inputs_vec = Vec::new();
            
            for i in 0..=max_input_port {
                // If multiple inputs on one port, we need to merge them?
                // For now, let's assume 1 input per port OR the node handles multiple receivers?
                // The trait takes Vec<Receiver>. Let's just flatten them in order of ports?
                // Actually, the trait signature `inputs: Vec<Receiver>` implies a list of inputs.
                // If we have ports, `inputs[0]` should be Port 0.
                // If Port 0 has multiple connections (Fan-In), we should probably merge them into ONE receiver 
                // BEFORE passing to the node, OR pass a Vec<Receiver> for Port 0?
                // To keep it simple: We will MERGE multiple edges to the same port into a single channel.
                
                if let Some(mut rxs) = node_input_map.remove(&i) {
                    if rxs.is_empty() {
                        // Create dummy channel? Or just don't push?
                        // If the node expects input at index i, we must provide it.
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

            // Prepare outputs
            let mut node_output_map = outputs.remove(&id).unwrap();
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
                        while let Some(_) = internal_rx.recv().await {}
                    });
                }
                node_outputs_vec.push(internal_tx);
            }

            set.spawn(async move {
                node.run(node_inputs_vec, node_outputs_vec).await
            });
        }

        while let Some(res) = set.join_next().await {
            res??;
        }

        Ok(())
    }
}
