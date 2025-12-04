use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{json, Value};
use anyhow::Result;
use reqwest::Client;
use std::collections::HashMap;

    pub struct HttpRequestNode {
    method: String,
    url_template: String,
    headers: HashMap<String, String>,
    body: Option<Value>,
    client: Client,
    retry_count: u32,
    retry_delay_ms: u64,
}

impl HttpRequestNode {
    pub fn new(
        method: String, 
        url_template: String, 
        headers: HashMap<String, String>, 
        body: Option<Value>,
        retry_count: u32,
        retry_delay_ms: u64,
    ) -> Self {
        Self {
            method: method.to_uppercase(),
            url_template,
            headers,
            body,
            client: Client::new(),
            retry_count,
            retry_delay_ms,
        }
    }

    fn resolve_url(&self, data: &Value) -> String {
        let mut url = self.url_template.clone();
        if let Some(obj) = data.as_object() {
            for (k, v) in obj {
                let placeholder = format!("{{{{ {} }}}}", k);
                let val_str = match v {
                    Value::String(s) => s.clone(),
                    _ => v.to_string(),
                };
                url = url.replace(&placeholder, &val_str);
                
                let placeholder_tight = format!("{{{{{}}}}}", k);
                url = url.replace(&placeholder_tight, &val_str);
            }
        }
        url
    }
}

#[async_trait]
impl StreamNode for HttpRequestNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(rx) = inputs.get_mut(0) {
            if let Some(tx) = outputs.first() {
                while let Some(data) = rx.recv().await {
                    let url = self.resolve_url(&data);
                    
                    let mut attempts = 0;
                    let max_attempts = self.retry_count + 1;
                    let mut last_error: Option<String> = None;
                    let mut success = false;

                    while attempts < max_attempts {
                        attempts += 1;
                        
                        let mut req_builder = match self.method.as_str() {
                            "GET" => self.client.get(&url),
                            "POST" => self.client.post(&url),
                            "PUT" => self.client.put(&url),
                            "DELETE" => self.client.delete(&url),
                            "PATCH" => self.client.patch(&url),
                            _ => self.client.get(&url),
                        };

                        for (k, v) in &self.headers {
                            req_builder = req_builder.header(k, v);
                        }

                        if let Some(body_template) = &self.body {
                            if body_template == &json!("$input") {
                                req_builder = req_builder.json(&data);
                            } else {
                                req_builder = req_builder.json(body_template);
                            }
                        }

                        match req_builder.send().await {
                            Ok(resp) => {
                                if resp.status().is_success() {
                                    let status = resp.status().as_u16();
                                    let headers: HashMap<String, String> = resp.headers()
                                        .iter()
                                        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                                        .collect();
                                    
                                    let body_bytes = resp.bytes().await.unwrap_or_default();
                                    let body_json: Value = match serde_json::from_slice(&body_bytes) {
                                        Ok(v) => v,
                                        Err(_) => {
                                            let text = String::from_utf8_lossy(&body_bytes).to_string();
                                            Value::String(text)
                                        }
                                    };

                                    let output_data = json!({
                                        "status": status,
                                        "headers": headers,
                                        "body": body_json,
                                        "original_input": data
                                    });

                                    tx.send(output_data).await?;
                                    success = true;
                                    break; // Success, exit retry loop
                                } else {
                                    // HTTP Error (4xx, 5xx)
                                    let status = resp.status();
                                    let error_msg = format!("HTTP Error: {}", status);
                                    last_error = Some(error_msg);
                                    // Continue to retry if attempts < max_attempts
                                }
                            }
                            Err(e) => {
                                last_error = Some(e.to_string());
                                // Continue to retry
                            }
                        }

                        if attempts < max_attempts {
                            let delay = self.retry_delay_ms;
                            if delay > 0 {
                                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                            }
                        }
                    }

                    if !success {
                        eprintln!("HTTP Request failed after {} attempts: {:?}", attempts, last_error);
                        let error_data = json!({
                            "error": last_error.unwrap_or_else(|| "Unknown error".to_string()),
                            "original_input": data
                        });
                        tx.send(error_data).await?;
                    }
                }
            }
        }
        Ok(())
    }
}
