use async_trait::async_trait;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::Value;
use anyhow::Result;

#[async_trait]
pub trait StreamNode: Send + Sync {
    /// Run the node logic.
    /// `inputs`: A list of input channels.
    /// `outputs`: A list of output channels. The node can write to any of them.
    async fn run(&self, inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()>;
}

pub mod executor;
pub mod nodes;
pub use executor::StreamExecutor;
