use crate::stream_engine::StreamNode;
use serde_json::Value;
use async_trait::async_trait;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub enum SelectOutputType {
    Auto,
    String,
    Number,
    Boolean,
    Json,
}

#[derive(Clone, Debug)]
pub struct SelectNode {
    template: String,
    output_type: SelectOutputType,
}

impl SelectNode {
    pub fn new(config: Value) -> Self {
        let template = config.get("template")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
            
        let output_type = match config.get("output_type").and_then(|v| v.as_str()) {
            Some("string") => SelectOutputType::String,
            Some("number") => SelectOutputType::Number,
            Some("boolean") => SelectOutputType::Boolean,
            Some("json") => SelectOutputType::Json,
            _ => SelectOutputType::Auto,
        };

        Self { template, output_type }
    }
}

#[async_trait]
impl StreamNode for SelectNode {
    async fn run(
        &self,
        mut inputs: Vec<mpsc::Receiver<Value>>,
        outputs: Vec<mpsc::Sender<Value>>,
    ) -> anyhow::Result<()> {
        let mut env = crate::stream_engine::expressions::create_environment();
        env.add_template("selection", &self.template)?;
        let tmpl = env.get_template("selection")?;

        if inputs.is_empty() {
             return Ok(());
        }
        
        let mut rx = inputs.remove(0);
        let tx = if !outputs.is_empty() {
            outputs[0].clone()
        } else {
             return Ok(());
        };

        while let Some(data) = rx.recv().await {
            match tmpl.render(&data) {
                Ok(rendered) => {
                    let output_value = match self.output_type {
                        SelectOutputType::Auto => {
                            match serde_json::from_str::<Value>(&rendered) {
                                Ok(json) => json,
                                Err(_) => Value::String(rendered),
                            }
                        },
                        SelectOutputType::String => Value::String(rendered),
                        SelectOutputType::Number => {
                            if let Ok(num) = rendered.trim().parse::<f64>() {
                                if let Some(n) = serde_json::Number::from_f64(num) {
                                    Value::Number(n)
                                } else {
                                     // Nan/Inf usually issue
                                     Value::Null // or error?
                                }
                            } else {
                                eprintln!("SelectNode: Failed to cast '{}' to Number", rendered);
                                continue; 
                            }
                        },
                        SelectOutputType::Boolean => {
                            // "true" / "false"
                             if let Ok(b) = rendered.trim().parse::<bool>() {
                                 Value::Bool(b)
                             } else {
                                 eprintln!("SelectNode: Failed to cast '{}' to Boolean", rendered);
                                 continue;
                             }
                        },
                        SelectOutputType::Json => {
                             match serde_json::from_str::<Value>(&rendered) {
                                Ok(json) => json,
                                Err(e) => {
                                    eprintln!("SelectNode: Failed to cast output to JSON: {}", e);
                                    continue;
                                }
                            }
                        }
                    };
                    
                    if tx.send(output_value).await.is_err() {
                         break;
                    }
                },
                Err(e) => {
                   eprintln!("SelectNode Template Error: {}", e);
                }
            }
        }
        
        Ok(())
    }
}
