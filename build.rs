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
    nodes: Vec<IntegrationNode>,
}

#[derive(Deserialize, Debug)]
struct IntegrationNode {
    name: String,
    #[serde(rename = "type")]
    node_type: String,
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
}

#[derive(Deserialize, Debug)]
struct HttpImplementation {
    method: String,
    url: String,
    headers: Option<HashMap<String, String>>,
    body: Option<Value>,
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
                                    let mut env = minijinja::Environment::new();

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
                        }
                    });
                }

                node_definitions.push(quote! {
                    crate::node_registry::NodeType {
                        id: #node_id.to_string(),
                        label: #label.to_string(),
                        category: #category.to_string(),
                        description: Some(#description.to_string()),
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


