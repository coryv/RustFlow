use std::env;
use std::fs;
use std::path::Path;
use serde::Deserialize;
use quote::{quote, format_ident};
use heck::{ToPascalCase, ToSnakeCase};
use std::collections::HashMap;
use serde_json::Value;

#[derive(Deserialize, Debug)]
struct Integration {
    name: String,
    #[serde(default)]
    credentials: Vec<CredentialProperty>,
    nodes: Vec<IntegrationNode>,
}

#[derive(Deserialize, Debug)]
struct CredentialProperty {
    name: String,
    label: String,
    #[serde(rename = "type")]
    property_type: String,
    #[serde(default)]
    required: bool,
    description: Option<String>,
}

#[derive(Deserialize, Debug)]
struct IntegrationNode {
    name: String,
    #[serde(rename = "type")]
    node_type: String,
    documentation: Option<String>,
    implementation: Implementation,
    #[serde(default)]
    properties: Vec<NodeProperty>,
}

#[derive(Deserialize, Debug)]
struct NodeProperty {
    name: String,
    label: String,
    #[serde(rename = "type")]
    property_type: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    options: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Implementation {
    Http(HttpImplementation),
    Polling(PollingImplementation),
}

#[derive(Deserialize, Debug)]
struct HttpImplementation {
    method: String,
    url: String,
    headers: Option<HashMap<String, String>>,
    body: Option<Value>,
}

#[derive(Deserialize, Debug)]
struct PollingImplementation {
    interval: String, // e.g. "60s"
    request: HttpImplementation,
    items_path: Option<String>, // JSON path to array of items, e.g. "results"
    dedupe_key: Option<String>, // Key to use for deduplication, e.g. "id"
}

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=integrations");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("integrations.rs");

    let integrations_dir = Path::new("integrations");
    if !integrations_dir.exists() {
        fs::write(&dest_path, "")?;
        return Ok(());
    }

    let mut integration_modules = Vec::new();
    let mut match_arms = Vec::new();
    let mut node_definitions = Vec::new();
    let mut integration_definitions = Vec::new();

    for entry in fs::read_dir(integrations_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            let content = fs::read_to_string(&path)?;
            let integration: Integration = serde_yaml::from_str(&content)?;
            
            let integration_name = &integration.name;
            let module_name = format_ident!("{}", integration_name.to_lowercase());
            
            let mut node_structs = Vec::new();

            for node in integration.nodes {
                let node_name_pascal = node.name.to_pascal_case();
                let struct_name = format_ident!("{}{}", integration_name.to_pascal_case(), node_name_pascal);
                
                // --- Implementation Generation ---
                match &node.implementation {
                    Implementation::Http(http) => {
                        let method = &http.method;
                        let url = &http.url;
                        let headers = http.headers.clone().unwrap_or_default();
                        
                        let body_template = match &http.body {
                            Some(Value::String(s)) => Some(s.clone()),
                            Some(v) => Some(serde_json::to_string(v)?),
                            None => None,
                        };

                        let headers_iter = headers.iter().map(|(k, v)| {
                            quote! {
                                headers.insert(#k.to_string(), #v.to_string());
                            }
                        });

                        let body_logic = if let Some(body) = body_template {
                            quote! {
                                let body_template = #body;
                                let body_rendered = env.render_str(body_template, &data)
                                    .map_err(|e| anyhow::anyhow!("Failed to render body template: {}", e))?;
                                let body_json: serde_json::Value = serde_json::from_str(&body_rendered)
                                    .map_err(|e| anyhow::anyhow!("Failed to parse rendered body as JSON: {}", e))?;
                                req_builder = req_builder.json(&body_json);
                            }
                        } else {
                            quote! {}
                        };

                        node_structs.push(quote! {
                            pub struct #struct_name {
                                client: reqwest::Client,
                            }

                            impl #struct_name {
                                pub fn new() -> Self {
                                    Self {
                                        client: reqwest::Client::new(),
                                    }
                                }
                            }

                            #[async_trait::async_trait]
                            impl crate::stream_engine::StreamNode for #struct_name {
                                async fn run(
                                    &self,
                                    mut inputs: Vec<tokio::sync::mpsc::Receiver<serde_json::Value>>,
                                    outputs: Vec<tokio::sync::mpsc::Sender<serde_json::Value>>,
                                ) -> anyhow::Result<()> {
                                    let env = minijinja::Environment::new();

                                    if let Some(rx) = inputs.get_mut(0) {
                                        if let Some(tx) = outputs.get(0) {
                                            while let Some(data) = rx.recv().await {
                                                let url_template = #url;
                                                let url = env.render_str(url_template, &data)
                                                    .map_err(|e| anyhow::anyhow!("Failed to render URL template: {}", e))?;

                                                let mut headers = std::collections::HashMap::new();
                                                #(#headers_iter)*
                                                
                                                let mut req_builder = match #method {
                                                    "GET" => self.client.get(&url),
                                                    "POST" => self.client.post(&url),
                                                    "PUT" => self.client.put(&url),
                                                    "DELETE" => self.client.delete(&url),
                                                    "PATCH" => self.client.patch(&url),
                                                    _ => self.client.get(&url),
                                                };

                                                for (k, v) in headers {
                                                    let v_rendered = env.render_str(&v, &data)
                                                        .map_err(|e| anyhow::anyhow!("Failed to render header {}: {}", k, e))?;
                                                    req_builder = req_builder.header(k, v_rendered);
                                                }

                                                #body_logic

                                                match req_builder.send().await {
                                                    Ok(resp) => {
                                                        let status = resp.status().as_u16();
                                                        let body_bytes = resp.bytes().await.unwrap_or_default();
                                                        let body_json: serde_json::Value = serde_json::from_slice(&body_bytes)
                                                            .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(&body_bytes).to_string()));

                                                        let output = serde_json::json!({
                                                            "status": status,
                                                            "body": body_json,
                                                            "original_input": data
                                                        });
                                                        
                                                        if let Err(e) = tx.send(output).await {
                                                            eprintln!("Failed to send output: {}", e);
                                                            break;
                                                        }
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Request failed: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Ok(())
                                }
                            }
                        });
                    }
                    Implementation::Polling(polling) => {
                        let interval_str = &polling.interval;
                        let interval_ms: u64 = if interval_str.ends_with('s') {
                            interval_str.trim_end_matches('s').parse::<u64>().unwrap_or(60) * 1000
                        } else if interval_str.ends_with("ms") {
                            interval_str.trim_end_matches("ms").parse::<u64>().unwrap_or(60000)
                        } else {
                            60000
                        };

                        let http = &polling.request;
                        let method = &http.method;
                        let url = &http.url;
                        let headers = http.headers.clone().unwrap_or_default();
                        let items_path = polling.items_path.clone();
                        let dedupe_key = polling.dedupe_key.clone();

                        let body_template = match &http.body {
                            Some(Value::String(s)) => Some(s.clone()),
                            Some(v) => Some(serde_json::to_string(v)?),
                            None => None,
                        };

                        let headers_iter = headers.iter().map(|(k, v)| {
                            quote! {
                                headers.insert(#k.to_string(), #v.to_string());
                            }
                        });

                        let body_logic = if let Some(body) = body_template {
                            quote! {
                                let body_template = #body;
                                let data = serde_json::Value::Null; 
                                let body_rendered = env.render_str(body_template, &data)
                                    .map_err(|e| anyhow::anyhow!("Failed to render body template: {}", e))?;
                                let body_json: serde_json::Value = serde_json::from_str(&body_rendered)
                                    .map_err(|e| anyhow::anyhow!("Failed to parse rendered body as JSON: {}", e))?;
                                req_builder = req_builder.json(&body_json);
                            }
                        } else {
                            quote! {}
                        };

                        let items_extraction = if let Some(path) = items_path {
                            quote! {
                                let items: Vec<serde_json::Value> = body_json.get(#path).and_then(|v| v.as_array()).cloned().unwrap_or_default();
                            }
                        } else {
                            quote! {
                                let items: Vec<serde_json::Value> = if body_json.is_array() {
                                    body_json.as_array().unwrap().clone()
                                } else {
                                    vec![body_json]
                                };
                            }
                        };

                        let dedupe_logic = if let Some(key) = dedupe_key {
                            quote! {
                                let item_id: Option<String> = item.get(#key).and_then(|v| v.as_str()).map(|s| s.to_string());
                                if let Some(id) = item_id {
                                    if seen_ids.contains(&id) {
                                        continue;
                                    }
                                    seen_ids.insert(id);
                                }
                            }
                        } else {
                            quote! {}
                        };

                        node_structs.push(quote! {
                            pub struct #struct_name {
                                client: reqwest::Client,
                            }

                            impl #struct_name {
                                pub fn new() -> Self {
                                    Self {
                                        client: reqwest::Client::new(),
                                    }
                                }
                            }

                            #[async_trait::async_trait]
                            impl crate::stream_engine::StreamNode for #struct_name {
                                async fn run(
                                    &self,
                                    mut inputs: Vec<tokio::sync::mpsc::Receiver<serde_json::Value>>,
                                    outputs: Vec<tokio::sync::mpsc::Sender<serde_json::Value>>,
                                ) -> anyhow::Result<()> {
                                    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
                                    let env: minijinja::Environment = minijinja::Environment::new();
                                    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(#interval_ms));

                                    if let Some(tx) = outputs.get(0) {
                                        loop {
                                            interval.tick().await;

                                            let data: serde_json::Value = serde_json::Value::Null;

                                            let url_template = #url;
                                            let url = env.render_str(url_template, &data)
                                                .map_err(|e| anyhow::anyhow!("Failed to render URL template: {}", e))?;

                                            let mut headers: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                                            #(#headers_iter)*
                                            
                                            let mut req_builder: reqwest::RequestBuilder = match #method {
                                                "GET" => self.client.get(&url),
                                                "POST" => self.client.post(&url),
                                                _ => self.client.get(&url),
                                            };

                                            for (k, v) in headers {
                                                let v_rendered = env.render_str(&v, &data)
                                                    .map_err(|e| anyhow::anyhow!("Failed to render header {}: {}", k, e))?;
                                                req_builder = req_builder.header(k, v_rendered);
                                            }

                                            #body_logic

                                            match req_builder.send().await {
                                                Ok(resp) => {
                                                    let body_bytes = resp.bytes().await.unwrap_or_default();
                                                    let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);

                                                    #items_extraction

                                                    for item in items {
                                                        #dedupe_logic
                                                        
                                                        if let Err(e) = tx.send(item).await {
                                                            eprintln!("Failed to send output: {}", e);
                                                            return Ok(());
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!("Polling request failed: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    Ok(())
                                }
                            }
                        });
                    }
                }

                // --- Registry Generation ---
                let node_name = &node.name;
                match_arms.push(quote! {
                    (#integration_name, #node_name) => Some(Box::new(#module_name::#struct_name::new())),
                });

                // --- NodeType Definition ---
                let node_id = format!("{}_{}", integration_name.to_snake_case(), node.name.to_snake_case());
                let label = format!("{}: {}", integration_name, node.name);
                let category = "Integration";
                let description = format!("{} integration", integration_name);
                let documentation = match &node.documentation {
                    Some(doc) => quote! { Some(#doc.to_string()) },
                    None => quote! { None },
                };

                let mut properties_code = Vec::new();
                for prop in node.properties {
                    let name = prop.name;
                    let label = prop.label;
                    let property_type = prop.property_type;
                    let required = prop.required;
                    let default = match prop.default {
                        Some(s) => quote! { Some(#s.to_string()) },
                        None => quote! { None },
                    };
                    let options = match prop.options {
                        Some(opts) => {
                            let opts_iter = opts.iter().map(|s| quote! { #s.to_string() });
                            quote! { Some(vec![#(#opts_iter),*]) }
                        },
                        None => quote! { None },
                    };

                    properties_code.push(quote! {
                        crate::node_registry::NodeProperty {
                            name: #name.to_string(),
                            label: #label.to_string(),
                            property_type: #property_type.to_string(),
                            options: #options,
                            default: #default,
                            required: #required,
                            json_schema: None,
                        }
                    });
                }

                node_definitions.push(quote! {
                    crate::node_registry::NodeType {
                        id: #node_id.to_string(),
                        label: #label.to_string(),
                        category: #category.to_string(),
                        description: Some(#description.to_string()),
                        documentation: #documentation,
                        properties: vec![#(#properties_code),*],
                    }
                });
            }

            integration_modules.push(quote! {
                pub mod #module_name {
                    use super::*;
                    #(#node_structs)*
                }
            });

            // --- Integration Definition ---
            let mut credential_props_code = Vec::new();
            for cred in integration.credentials {
                let name = cred.name;
                let label = cred.label;
                let property_type = cred.property_type;
                let required = cred.required;
                let description = match cred.description {
                    Some(s) => quote! { Some(#s.to_string()) },
                    None => quote! { None },
                };

                credential_props_code.push(quote! {
                    crate::integration_registry::CredentialProperty {
                        name: #name.to_string(),
                        label: #label.to_string(),
                        property_type: #property_type.to_string(),
                        required: #required,
                        description: #description,
                    }
                });
            }

            let integration_description = format!("{} integration", integration_name);
            integration_definitions.push(quote! {
                crate::integration_registry::IntegrationDefinition {
                    name: #integration_name.to_string(),
                    description: #integration_description.to_string(),
                    credentials: vec![#(#credential_props_code),*],
                }
            });
        }
    }

    let output = quote! {
        #(#integration_modules)*

        pub fn create_integration_node(integration: &str, node: &str) -> Option<Box<dyn crate::stream_engine::StreamNode>> {
            match (integration, node) {
                #(#match_arms)*
                _ => None,
            }
        }

        pub fn get_integration_node_definitions() -> Vec<crate::node_registry::NodeType> {
            vec![
                #(#node_definitions),*
            ]
        }
    };

    fs::write(&dest_path, output.to_string())?;
    Ok(())
}


