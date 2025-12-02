use crate::stream_engine::StreamNode;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::mpsc::{Receiver, Sender};
use anyhow::{Result, anyhow};
use scraper::{Html, Selector};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractMode {
    Text,
    Html,
    Attribute(String),
}

pub struct HtmlExtractNode {
    selector: String,
    mode: ExtractMode,
}

impl HtmlExtractNode {
    pub fn new(selector: String, mode: ExtractMode) -> Self {
        Self { selector, mode }
    }
}

#[async_trait]
impl StreamNode for HtmlExtractNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, mut outputs: Vec<Sender<Value>>) -> Result<()> {
        let output = outputs.remove(0);
        let mut rx = inputs.remove(0);

        let selector = Selector::parse(&self.selector)
            .map_err(|e| anyhow!("Invalid CSS selector: {:?}", e))?;

        while let Some(input) = rx.recv().await {
            // Expect input to be an object with an "html" or "content" field, or just a string
            let html_content = if let Some(s) = input.as_str() {
                s
            } else if let Some(s) = input.get("html").and_then(|v| v.as_str()) {
                s
            } else if let Some(s) = input.get("content").and_then(|v| v.as_str()) {
                s
            } else if let Some(s) = input.get("body").and_then(|v| v.as_str()) {
                 s
            } else {
                // If input is an object but doesn't have obvious html fields, try converting the whole thing to string?
                // Or just skip/error. Let's skip with a warning for now.
                eprintln!("HtmlExtractNode: Input does not contain string content");
                continue;
            };

            let extracted_values = {
                let document = Html::parse_document(html_content);
                let mut values = Vec::new();

                for element in document.select(&selector) {
                    let value = match &self.mode {
                        ExtractMode::Text => element.text().collect::<Vec<_>>().join(" "),
                        ExtractMode::Html => element.html(),
                        ExtractMode::Attribute(attr) => element.value().attr(attr).unwrap_or("").to_string(),
                    };
                    values.push(value);
                }
                values
            };

            // Emit result. If we found multiple matches, emit an array? Or emit one item per match?
            // For now, let's emit an object with "extracted": [values]
            // Better yet, preserve original input and add "extracted" field.
            
            let mut result_obj = if let Some(obj) = input.as_object() {
                obj.clone()
            } else {
                serde_json::Map::new()
            };
            
            result_obj.insert("extracted".to_string(), json!(extracted_values));
            
            if output.send(Value::Object(result_obj)).await.is_err() {
                break;
            }
        }

        Ok(())
    }
}
