use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};
use std::fs;
use std::path::Path;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct IntegrationDef {
    name: String,
    nodes: Vec<NodeDef>,
}

#[derive(Deserialize)]
struct NodeDef {
    name: String,
    #[serde(rename = "type")]
    node_type: String,
    implementation: ImplementationDef,
}

#[derive(Deserialize)]
struct ImplementationDef {
    #[serde(rename = "type")]
    impl_type: String,
    method: String,
    url: String,
    headers: Option<HashMap<String, String>>,
    body: Option<HashMap<String, serde_yaml::Value>>,
}

#[proc_macro]
pub fn generate_integration(input: TokenStream) -> TokenStream {
    let file_path = parse_macro_input!(input as LitStr).value();
    
    // Read YAML file
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let full_path = Path::new(&manifest_dir).join(&file_path);
    let content = match fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(e) => return syn::Error::new(
            proc_macro2::Span::call_site(), 
            format!("Failed to read integration file at {:?}: {}", full_path, e)
        ).to_compile_error().into(),
    };

    let integration: IntegrationDef = match serde_yaml::from_str(&content) {
        Ok(d) => d,
        Err(e) => return syn::Error::new(
            proc_macro2::Span::call_site(), 
            format!("Failed to parse integration YAML: {}", e)
        ).to_compile_error().into(),
    };

    let mut generated_structs = Vec::new();

    for node in integration.nodes {
        let struct_name = format!("{}{}", integration.name, node.name.replace(" ", ""));
        let struct_ident = syn::Ident::new(&struct_name, proc_macro2::Span::call_site());
        
        let method = node.implementation.method;
        let url_template = node.implementation.url;
        
        // Headers handling
        let headers_code = if let Some(headers) = node.implementation.headers {
            let mut inserts = Vec::new();
            for (k, v) in headers {
                inserts.push(quote! {
                    headers.insert(#k.to_string(), #v.to_string());
                });
            }
            quote! {
                let mut headers = std::collections::HashMap::new();
                #(#inserts)*
            }
        } else {
            quote! { let headers = std::collections::HashMap::new(); }
        };

        // Body handling
        let body_code = if let Some(body) = node.implementation.body {
             let mut inserts = Vec::new();
            for (k, v) in body {
                // Convert YAML value to JSON string representation to embed in code
                // We use serde_json::to_string to get a valid JSON string of the value
                // Then in the generated code, we parse it back to serde_json::Value
                let json_str = serde_json::to_string(&v).unwrap();
                inserts.push(quote! {
                    let val: serde_json::Value = serde_json::from_str(#json_str).unwrap();
                    body_map.insert(#k.to_string(), val);
                });
            }
            quote! {
                let mut body_map = serde_json::Map::new();
                #(#inserts)*
                let body = Some(serde_json::Value::Object(body_map));
            }
        } else {
            quote! { let body = None; }
        };

        generated_structs.push(quote! {
            pub struct #struct_ident {
                http_node: crate::stream_engine::nodes::HttpRequestNode,
            }

            impl #struct_ident {
                pub fn new() -> Self {
                    #headers_code
                    #body_code
                    
                    Self {
                        http_node: crate::stream_engine::nodes::HttpRequestNode::new(
                            #method.to_string(),
                            #url_template.to_string(),
                            headers,
                            body
                        )
                    }
                }
            }

            #[async_trait::async_trait]
            impl crate::stream_engine::StreamNode for #struct_ident {
                async fn run(
                    &self,
                    inputs: Vec<tokio::sync::mpsc::Receiver<serde_json::Value>>,
                    outputs: Vec<tokio::sync::mpsc::Sender<serde_json::Value>>,
                ) -> anyhow::Result<()> {
                    // Delegate to HttpRequestNode
                    self.http_node.run(inputs, outputs).await
                }
            }
        });
    }

    let output = quote! {
        #(#generated_structs)*
    };

    output.into()
}
