use crate::stream_engine::StreamNode;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc::{Receiver, Sender};
use anyhow::Result;

pub struct IntegrationNode {
    inner: Box<dyn StreamNode>,
}

impl IntegrationNode {
    pub fn new(integration: &str, node: &str) -> Result<Self> {
        let inner = crate::integrations::create_integration_node(integration, node)
            .ok_or_else(|| anyhow::anyhow!("Integration node not found: {}/{}", integration, node))?;
        Ok(Self { inner })
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
