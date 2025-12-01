use crate::stream_engine::StreamNode;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc::{Receiver, Sender};
use anyhow::Result;

// This is a placeholder for the registry.
// In a real implementation, we'd use `inventory` or `linkme` crates for distributed registration.
// For MVP, we will manually match on the name in the factory, or use this node to wrap the generated struct if we can instantiate it dynamically.
// Actually, since the macro generates a struct, we need a way to map "SlackPostMessage" string to `SlackPostMessage` struct at runtime.
// The easiest way without reflection is to have the macro ALSO generate a match arm in a central `get_integration_node` function.

// But since we can't easily modify a central function from a macro in another crate, 
// we will skip the "Generic IntegrationNode" for now and just use the generated structs directly in the Schema Loader?
// No, the Schema Loader needs to know about them.

// Alternative: The macro generates a `register_integrations()` function that returns a Map<String, Box<dyn Fn() -> Box<dyn StreamNode>>>.
// The user calls this function in `main.rs` and passes the map to the Loader.

pub struct IntegrationNode {
    inner: Box<dyn StreamNode>,
}

impl IntegrationNode {
    pub fn new(inner: Box<dyn StreamNode>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl StreamNode for IntegrationNode {
    async fn run(
        &self,
        inputs: Vec<Receiver<Value>>,
        outputs: Vec<Sender<Value>>,
    ) -> Result<()> {
        self.inner.run(inputs, outputs).await
    }
}
