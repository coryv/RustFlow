use std::collections::HashMap;
use serde_json::Value;
use anyhow::{Result, anyhow};
use crate::stream_engine::StreamNode;
use crate::stream_engine::nodes;
use crate::integrations;

type NodeCreator = Box<dyn Fn(Value, &HashMap<String, String>) -> Result<Box<dyn StreamNode>> + Send + Sync>;

pub struct NodeFactory {
    creators: HashMap<String, NodeCreator>,
}

impl Default for NodeFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeFactory {
    pub fn new() -> Self {
        let mut factory = Self {
            creators: HashMap::new(),
        };
        factory.register_defaults();
        factory
    }

    pub fn register<F>(&mut self, type_name: &str, creator: F)
    where
        F: Fn(Value, &HashMap<String, String>) -> Result<Box<dyn StreamNode>> + Send + Sync + 'static,
    {
        self.creators.insert(type_name.to_string(), Box::new(creator));
    }

    pub fn create(&self, type_name: &str, config: Value, secrets: &HashMap<String, String>) -> Result<Box<dyn StreamNode>> {
        if let Some(creator) = self.creators.get(type_name) {
            return creator(config, secrets);
        }

        // Fallback for integrations
        if let Some(node) = integrations::create_node_by_id(type_name) {
            return Ok(node);
        }

        Err(anyhow!("Unknown node type: {}", type_name))
    }

    fn register_defaults(&mut self) {
        self.register("manual_trigger", |_, _| Ok(Box::new(nodes::ManualTrigger)));
        self.register("console_output", |_, _| Ok(Box::new(nodes::ConsoleOutput)));
        self.register("set_data", |config, _| Ok(Box::new(nodes::SetData::new(config))));
        
        self.register("router", |config, _| {
            let key = config.get("key").and_then(|v| v.as_str()).unwrap_or("id");
            let value = config.get("value").cloned().unwrap_or(Value::Null);
            let operator = config.get("operator").and_then(|v| v.as_str()).unwrap_or("==").to_string();
            Ok(Box::new(nodes::RouterNode::new(key.to_string(), value, operator)))
        });

        self.register("join", |config, _| {
            let join_type_str = config.get("type").and_then(|v| v.as_str()).unwrap_or("index");
            let mode_str = config.get("mode").and_then(|v| v.as_str()).unwrap_or("inner");
            
            let mode = match mode_str {
                "left" => nodes::JoinMode::Left,
                "right" => nodes::JoinMode::Right,
                "outer" => nodes::JoinMode::Outer,
                _ => nodes::JoinMode::Inner,
            };

            let join_type = match join_type_str {
                "key" => {
                    let key_str = config.get("key").and_then(|v| v.as_str()).unwrap_or("id");
                    let right_key_str = config.get("right_key").and_then(|v| v.as_str()).unwrap_or(key_str);
                    
                    // Split by comma and trim to support composite keys
                    let left_keys: Vec<String> = key_str.split(',').map(|s| s.trim().to_string()).collect();
                    let right_keys: Vec<String> = right_key_str.split(',').map(|s| s.trim().to_string()).collect();
                    
                    nodes::JoinType::Key(left_keys, right_keys)
                },
                _ => nodes::JoinType::Index,
            };
            Ok(Box::new(nodes::JoinNode::new(join_type, mode)))
        });

        self.register("file_source", |config, _| {
            let path = config.get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'path' for file_source"))?;
            Ok(Box::new(nodes::FileSource::new(path.to_string())))
        });

        self.register("union", |config, _| {
            let mode_str = config.get("mode").and_then(|v| v.as_str()).unwrap_or("interleaved");
            let mode = match mode_str {
                "sequential" => nodes::UnionMode::Sequential,
                _ => nodes::UnionMode::Interleaved,
            };
            Ok(Box::new(nodes::UnionNode::new(mode)))
        });

        self.register("http_request", |config, _| {
            let method = config.get("method").and_then(|v| v.as_str()).unwrap_or("GET").to_string();
            let url = config.get("url").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing 'url' for http_request"))?.to_string();
            
            let headers_val = config.get("headers").cloned().unwrap_or(serde_json::json!({}));
            let mut headers = std::collections::HashMap::new();
            if let Some(h_obj) = headers_val.as_object() {
                for (k, v) in h_obj {
                    if let Some(s) = v.as_str() {
                        headers.insert(k.clone(), s.to_string());
                    }
                }
            }

            let body = config.get("body").cloned();
            let retry_count = config.get("retry_count").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let retry_delay_ms = config.get("retry_delay_ms").and_then(|v| v.as_u64()).unwrap_or(0);
            
            Ok(Box::new(nodes::HttpRequestNode::new(method, url, headers, body, retry_count, retry_delay_ms)))
        });

        self.register("time_trigger", |config, _| {
            let cron = config.get("cron").and_then(|v| v.as_str()).unwrap_or("0 * * * * *").to_string();
            let interval = config.get("interval").and_then(|v| v.as_u64()).unwrap_or(1);
            Ok(Box::new(nodes::TimeTrigger::new(cron, interval)))
        });

        self.register("webhook_trigger", |config, _| {
            let path = config.get("path").and_then(|v| v.as_str()).unwrap_or("/").to_string();
            let method = config.get("method").and_then(|v| v.as_str()).unwrap_or("GET").to_string();
            Ok(Box::new(nodes::WebhookTrigger::new(path, method)))
        });

        self.register("code", |config, _| {
            let lang = config.get("lang").and_then(|v| v.as_str()).unwrap_or("js").to_string();
            let code = config.get("code").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing 'code' for code node"))?.to_string();
            Ok(Box::new(nodes::CodeNode::new(lang, code)))
        });

        self.register("delay", |config, _| {
            Ok(Box::new(nodes::DelayNode::new(config)))
        });

        self.register("html_extract", |config, _| {
            let selector = config.get("selector").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing 'selector' for html_extract"))?.to_string();
            let mode_str = config.get("mode").and_then(|v| v.as_str()).unwrap_or("text");
            let mode = match mode_str {
                "html" => nodes::ExtractMode::Html,
                "attribute" => {
                    let attr = config.get("attribute").and_then(|v| v.as_str()).unwrap_or("href").to_string();
                    nodes::ExtractMode::Attribute(attr)
                },
                _ => nodes::ExtractMode::Text,
            };
            Ok(Box::new(nodes::HtmlExtractNode::new(selector, mode)))
        });

        self.register("dedupe", |config, _| {
            let key = config.get("key").and_then(|v| v.as_str()).map(|s| s.to_string());
            Ok(Box::new(nodes::DedupeNode::new(key)))
        });

        self.register("split", |config, _| {
            let path = config.get("path").and_then(|v| v.as_str()).map(|s| s.to_string());
            Ok(Box::new(nodes::SplitNode::new(path)))
        });

        self.register("accumulate", |_, _| {
            Ok(Box::new(nodes::AccumulateNode::new()))
        });

        self.register("agent", |config, secrets| {
            let model = config.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4o").to_string();
            let system_prompt = config.get("system_prompt").and_then(|v| v.as_str()).unwrap_or("You are a helpful AI assistant.").to_string();
            let user_prompt = config.get("user_prompt").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let api_key = config.get("api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
            let credential_id = config.get("credential_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let api_base = config.get("api_base").and_then(|v| v.as_str()).map(|s| s.to_string());
            let json_schema = config.get("json_schema").cloned();
            let provider = config.get("provider").and_then(|v| v.as_str()).unwrap_or("openai").to_string();
            
            // Resolve API Key from secrets if credential_id is present
            let mut final_api_key = api_key;
            if let Some(ref cred_id) = credential_id {
                if let Some(secret) = secrets.get(cred_id) {
                    final_api_key = Some(secret.clone());
                }
            }

            Ok(Box::new(nodes::AgentNode {
                model,
                system_prompt,
                user_prompt,
                api_key: final_api_key,
                credential_id,
                api_base,
                json_schema,
                provider,
            }))
        });
    }
}
