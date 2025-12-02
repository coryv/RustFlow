use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use anyhow::{Result, anyhow};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub nodes: Vec<NodeDefinition>,
    pub edges: Vec<EdgeDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default, alias = "config")]
    pub data: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EdgeDefinition {
    pub from: String,
    #[serde(default)]
    pub from_port: usize,
    pub to: String,
    #[serde(default)]
    pub to_port: usize,
}

use crate::stream_engine::{StreamExecutor, StreamNode, nodes};
use crate::stream_engine::nodes::agent_node::AgentNode;
use crate::stream_engine::nodes::{HtmlExtractNode, ExtractMode};
use crate::integrations;

impl WorkflowDefinition {
    pub fn to_executor(&self, secrets: &std::collections::HashMap<String, String>) -> Result<StreamExecutor> {
        let mut executor = StreamExecutor::new();

        for node_def in &self.nodes {
            let node: Box<dyn StreamNode> = match node_def.node_type.as_str() {
                "manual_trigger" => Box::new(nodes::ManualTrigger),
                "console_output" => Box::new(nodes::ConsoleOutput),
                "set_data" => {
                    Box::new(nodes::SetData::new(node_def.data.clone()))
                },
                "router" => {
                    let key = node_def.data.get("key").and_then(|v| v.as_str()).unwrap_or("id");
                    let value = node_def.data.get("value").cloned().unwrap_or(Value::Null);
                    Box::new(nodes::RouterNode::new(key.to_string(), value))
                },
                "join" => {
                    let join_type_str = node_def.data.get("type").and_then(|v| v.as_str()).unwrap_or("index");
                    let join_type = match join_type_str {
                        "key" => {
                            let key = node_def.data.get("key").and_then(|v| v.as_str()).unwrap_or("id");
                            let right_key = node_def.data.get("right_key").and_then(|v| v.as_str()).unwrap_or(key);
                            nodes::JoinType::Key(key.to_string(), right_key.to_string())
                        },
                        _ => nodes::JoinType::Index,
                    };
                    Box::new(nodes::JoinNode::new(join_type))
                },
                "file_source" => {
                    let path = node_def.data.get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow!("Missing 'path' for file_source"))?;
                    Box::new(nodes::FileSource::new(path.to_string()))
                },
                "union" => {
                    let mode_str = node_def.data.get("mode").and_then(|v| v.as_str()).unwrap_or("interleaved");
                    let mode = match mode_str {
                        "sequential" => nodes::UnionMode::Sequential,
                        _ => nodes::UnionMode::Interleaved,
                    };
                    Box::new(nodes::UnionNode::new(mode))
                },
                "http_request" => {
                    let method = node_def.data.get("method").and_then(|v| v.as_str()).unwrap_or("GET").to_string();
                    let url = node_def.data.get("url").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing 'url' for http_request"))?.to_string();
                    
                    let headers_val = node_def.data.get("headers").cloned().unwrap_or(json!({}));
                    let mut headers = std::collections::HashMap::new();
                    if let Some(h_obj) = headers_val.as_object() {
                        for (k, v) in h_obj {
                            if let Some(s) = v.as_str() {
                                headers.insert(k.clone(), s.to_string());
                            }
                        }
                    }

                    let body = node_def.data.get("body").cloned();
                    
                    Box::new(nodes::HttpRequestNode::new(method, url, headers, body))
                },
                "time_trigger" => {
                    let cron = node_def.data.get("cron").and_then(|v| v.as_str()).unwrap_or("0 * * * * *").to_string();
                    let interval = node_def.data.get("interval").and_then(|v| v.as_u64()).unwrap_or(1);
                    Box::new(nodes::TimeTrigger::new(cron, interval))
                },
                "webhook_trigger" => {
                    let path = node_def.data.get("path").and_then(|v| v.as_str()).unwrap_or("/").to_string();
                    let method = node_def.data.get("method").and_then(|v| v.as_str()).unwrap_or("GET").to_string();
                    Box::new(nodes::WebhookTrigger::new(path, method))
                },
                "code" => {
                    let lang = node_def.data.get("lang").and_then(|v| v.as_str()).unwrap_or("js").to_string();
                    let code = node_def.data.get("code").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing 'code' for code node"))?.to_string();
                    Box::new(nodes::CodeNode::new(lang, code))
                },
                "agent" => {
                    let model = node_def.data.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4o").to_string();
                    let system_prompt = node_def.data.get("system_prompt").and_then(|v| v.as_str()).unwrap_or("You are a helpful AI assistant.").to_string();
                    let user_prompt = node_def.data.get("user_prompt").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let api_key = node_def.data.get("api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let credential_id = node_def.data.get("credential_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let api_base = node_def.data.get("api_base").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let json_schema = node_def.data.get("json_schema").cloned();
                    let provider = node_def.data.get("provider").and_then(|v| v.as_str()).unwrap_or("openai").to_string();
                    
                    // Resolve API Key from secrets if credential_id is present
                    let mut final_api_key = api_key;
                    if let Some(ref cred_id) = credential_id {
                        if let Some(secret) = secrets.get(cred_id) {
                            final_api_key = Some(secret.clone());
                        }
                    }

                    Box::new(AgentNode {
                        model,
                        system_prompt,
                        user_prompt,
                        api_key: final_api_key,
                        credential_id,
                        api_base,
                        json_schema,
                        provider,
                    })
                },
                "slack_post_message" => {
                    integrations::create_integration_node("Slack", "Post Message")
                        .ok_or_else(|| anyhow!("Integration node Slack/Post Message not found"))?
                },
                "notion_create_page" => {
                    integrations::create_integration_node("Notion", "Create Page")
                        .ok_or_else(|| anyhow!("Integration node Notion/Create Page not found"))?
                },
                "html_extract" => {
                    let selector = node_def.data.get("selector").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing 'selector' for html_extract"))?.to_string();
                    let mode_str = node_def.data.get("mode").and_then(|v| v.as_str()).unwrap_or("text");
                    let mode = match mode_str {
                        "html" => ExtractMode::Html,
                        "attribute" => {
                            let attr = node_def.data.get("attribute").and_then(|v| v.as_str()).unwrap_or("href").to_string();
                            ExtractMode::Attribute(attr)
                        },
                        _ => ExtractMode::Text,
                    };
                    Box::new(HtmlExtractNode::new(selector, mode))
                },
                _ => return Err(anyhow!("Unknown node type: {}", node_def.node_type)),
            };
            executor.add_node(node_def.id.clone(), node);
        }

        for edge_def in &self.edges {
            executor.add_connection(edge_def.from.clone(), edge_def.from_port, edge_def.to.clone(), edge_def.to_port);
        }

        Ok(executor)
    }
}

pub struct WorkflowLoader;

impl WorkflowLoader {
    pub fn new() -> Self {
        Self
    }

    pub fn load(&self, content: &str) -> Result<WorkflowDefinition> {
        // Try parsing as YAML (which is a superset of JSON)
        let def: WorkflowDefinition = serde_yaml::from_str(content)?;
        Ok(def)
    }
}
