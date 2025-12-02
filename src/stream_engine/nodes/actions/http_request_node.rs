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
}

impl HttpRequestNode {
    pub fn new(method: String, url_template: String, headers: HashMap<String, String>, body: Option<Value>) -> Self {
        Self {
            method: method.to_uppercase(),
            url_template,
            headers,
            body,
            client: Client::new(),
        }
    }

    fn resolve_url(&self, data: &Value) -> String {
        let mut url = self.url_template.clone();
        if let Some(obj) = data.as_object() {
            for (k, v) in obj {
                let placeholder = format!("{{{{ {} }}}}", k);
                // Simple string replacement. 
                // Note: In a real system, we'd want a more robust template engine (e.g. Handlebars/Tera).
                // But for MVP, this works.
                // We handle both string and non-string values by converting to string.
                let val_str = match v {
                    Value::String(s) => s.clone(),
                    _ => v.to_string(),
                };
                url = url.replace(&placeholder, &val_str);
                
                // Also try without spaces {{key}}
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
            if let Some(tx) = outputs.get(0) {
                while let Some(data) = rx.recv().await {
                    let url = self.resolve_url(&data);
                    
                    let mut req_builder = match self.method.as_str() {
                        "GET" => self.client.get(&url),
                        "POST" => self.client.post(&url),
                        "PUT" => self.client.put(&url),
                        "DELETE" => self.client.delete(&url),
                        "PATCH" => self.client.patch(&url),
                        _ => self.client.get(&url), // Default to GET
                    };

                    // Add headers
                    for (k, v) in &self.headers {
                        req_builder = req_builder.header(k, v);
                    }

                    // Add body
                    if let Some(body_template) = &self.body {
                        // If body is "Use Input", use the input data
                        // Otherwise use the static body (maybe templated later?)
                        // For now, let's assume static body or input.
                        // If body_template is a string "$input", use data.
                        if body_template == &json!("$input") {
                            req_builder = req_builder.json(&data);
                        } else {
                            req_builder = req_builder.json(body_template);
                        }
                    }

                    // Send request
                    // We capture errors but don't crash the node, instead emit error object?
                    // For now, let's just log and continue or emit error structure.
                    match req_builder.send().await {
                        Ok(resp) => {
                            let status = resp.status().as_u16();
                            let headers: HashMap<String, String> = resp.headers()
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                                .collect();
                            
                            let body_bytes = resp.bytes().await.unwrap_or_default();
                            let body_json: Value = match serde_json::from_slice(&body_bytes) {
                                Ok(v) => v,
                                Err(_) => {
                                    // Fallback to string
                                    let text = String::from_utf8_lossy(&body_bytes).to_string();
                                    Value::String(text)
                                }
                            };

                            let output_data = json!({
                                "status": status,
                                "headers": headers,
                                "body": body_json,
                                "original_input": data // Pass through input context? Optional but helpful.
                            });

                            tx.send(output_data).await?;
                        }
                        Err(e) => {
                            eprintln!("HTTP Request failed: {}", e);
                            let error_data = json!({
                                "error": e.to_string(),
                                "original_input": data
                            });
                            tx.send(error_data).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
