use wasm_bindgen::prelude::*;
use rust_flow::engine::{Executor, TaskSpawner, Workflow};
use rust_flow::nodes::{ManualTrigger, SetData, ConsoleOutput};
use std::future::Future;
use std::pin::Pin;
use serde_json::Value;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

pub struct WasmSpawner;

impl TaskSpawner for WasmSpawner {
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[wasm_bindgen]
pub struct WasmWorkflow {
    inner: Option<Workflow>,
}

#[wasm_bindgen]
impl WasmWorkflow {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self { inner: Some(Workflow::new()) }
    }

    pub fn add_manual_trigger(&mut self, id: String) {
        if let Some(w) = self.inner.as_mut() {
            w.add_node(id, Box::new(ManualTrigger));
        }
    }

    pub fn add_set_data(&mut self, id: String, json_data: String) -> Result<(), JsValue> {
        if let Some(w) = self.inner.as_mut() {
            let data: Value = serde_json::from_str(&json_data)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            w.add_node(id, Box::new(SetData::new(data)));
        }
        Ok(())
    }

    pub fn add_console_output(&mut self, id: String) {
        if let Some(w) = self.inner.as_mut() {
            w.add_node(id, Box::new(ConsoleOutput));
        }
    }

    pub fn add_time_trigger(&mut self, id: String, cron: String) {
        if let Some(w) = self.inner.as_mut() {
            w.add_node(id, Box::new(rust_flow::nodes::TimeTrigger::new(cron)));
        }
    }

    pub fn add_webhook_trigger(&mut self, id: String, path: String, method: String) {
        if let Some(w) = self.inner.as_mut() {
            w.add_node(id, Box::new(rust_flow::nodes::WebhookTrigger::new(path, method)));
        }
    }

    pub fn add_connection(&mut self, from: String, to: String) {
        if let Some(w) = self.inner.as_mut() {
            w.add_connection(from, to);
        }
    }

    pub async fn run(&mut self) -> Result<(), JsValue> {
        // We take the workflow out to run it (since Executor needs ownership or Arc)
        // For simplicity, we clone or just take it. Executor takes ownership.
        if let Some(workflow) = self.inner.take() {
            let executor = Executor::new(workflow, Box::new(WasmSpawner));
            executor.run().await
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            // Note: Workflow is consumed. In a real app we might want to keep it or clone it.
            // But Workflow nodes are Box<dyn Node> which isn't Clone.
            // So we'd need to rebuild or change architecture to Arc<Node>.
            // For now, "run once" is fine.
        } else {
            return Err(JsValue::from_str("Workflow already ran or invalid"));
        }
        Ok(())
    }
}
