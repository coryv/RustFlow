use crate::stream_engine::StreamNode;
use crate::stream_engine::utils::loose_eq;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct SwitchNode {
    expression: String,
    cases: Vec<Value>,
}

impl SwitchNode {
    pub fn new(config: Value) -> Self {
        let expression = config.get("expression").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let cases = config.get("cases").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        Self { expression, cases }
    }
}

#[async_trait]
impl StreamNode for SwitchNode {
    async fn run(
        &self,
        mut inputs: Vec<mpsc::Receiver<Value>>,
        outputs: Vec<mpsc::Sender<Value>>,
    ) -> anyhow::Result<()> {
        if inputs.is_empty() { return Ok(()); }
        let mut rx = inputs.remove(0);

        while let Some(input) = rx.recv().await {
             let env = crate::stream_engine::expressions::create_environment();
             // Render expression to get the value to switch on
             let val_str = match env.render_str(&self.expression, &input) {
                 Ok(s) => s,
                 Err(e) => {
                     eprintln!("Switch Expression Error: {}", e);
                     continue;
                 }
             };

             // Try to parse rendered string as JSON value for typed comparison if possible,
             // otherwise treat as string.
             // Minijinja renders to string. If user wants boolean "true", it renders as "true".
             // `loose_eq` handles "true" == true.
             // But if user expects number comparison, we need to be careful.
             // Let's assume the rendered string is the value we verify against.
             // We can wrap it in Value::String.
             let actual_val = Value::String(val_str);

             let mut matched_index = None;

             for (i, case_val) in self.cases.iter().enumerate() {
                 if loose_eq(&actual_val, case_val) {
                     matched_index = Some(i);
                     break;
                 }
             }

             // Output selection
             // If matched, index `i`.
             // If not matched, index `cases.len()` (Default).
             // Verify output port exists.
             
             let params_index = matched_index.unwrap_or(self.cases.len());
             
             if let Some(tx) = outputs.get(params_index) {
                 if tx.send(input.clone()).await.is_err() {
                     // Could be closed, continue processing?
                 }
             } else {
                 // No output port for this case (or default).
                 // Just drop.
             }
        }
        Ok(())
    }
}
