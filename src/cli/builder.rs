use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input, Select, Confirm};
use console::style;
use rust_flow::schema::{WorkflowDefinition, NodeDefinition, EdgeDefinition};
use rust_flow::node_registry::{get_node_registry, NodeType, NodeProperty};
use std::collections::HashMap;
use serde_json::Value;
use std::fs;

pub struct BuilderState {
    nodes: Vec<NodeDefinition>,
    edges: Vec<EdgeDefinition>,
}

impl BuilderState {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let theme = ColorfulTheme::default();
        loop {
            // Clear screen or just print separator
            println!("\n{}", style("--- Workflow Builder ---").bold());
            self.print_workflow_status();
            println!("{}", style("------------------------").bold());

            let choices = vec![
                "Add Node",
                "Connect Nodes",
                "List Nodes", // Keep for raw detail if needed, or remove since we have status
                "Test Node",
                "Test Workflow",
                "Save Workflow",
                "Quit",
            ];

            let selection = Select::with_theme(&theme)
                .with_prompt("Select Action")
                .default(0)
                .items(&choices)
                .interact()?;

            match selection {
                0 => self.add_node()?,
                1 => self.connect_nodes()?,
                2 => self.list_nodes(),
                3 => self.test_node().await?,
                4 => self.test_workflow().await?,
                5 => self.save_workflow()?,
                6 => break,
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    fn add_node(&mut self) -> Result<()> {
        let registry = get_node_registry();
        let theme = ColorfulTheme::default();
        
        // Group by category
        let mut categories: Vec<String> = registry.iter().map(|n| n.category.clone()).collect();
        categories.sort();
        categories.dedup();

        if self.nodes.is_empty() {
            println!("{}", style("First node must be a Trigger.").yellow());
            categories.retain(|c| c == "Trigger");
        }

        let category_idx = Select::with_theme(&theme)
            .with_prompt("Select Category")
            .items(&categories)
            .interact()?;
        
        let category = &categories[category_idx];
        let nodes_in_cat: Vec<&NodeType> = registry.iter().filter(|n| &n.category == category).collect();
        
        let node_labels: Vec<String> = nodes_in_cat.iter().map(|n| n.label.clone()).collect();
        let node_idx = Select::with_theme(&theme)
            .with_prompt("Select Node Type")
            .items(&node_labels)
            .interact()?;

        let node_type = nodes_in_cat[node_idx];
        
        let mut input = Input::with_theme(&theme);
        input = input.with_prompt("Node ID (unique name)");
        input = input.validate_with(|input: &String| -> Result<(), &str> {
                if input.is_empty() { Err("ID cannot be empty") }
                else if self.nodes.iter().any(|n| &n.id == input) { Err("ID already exists") }
                else { Ok(()) }
            });
        let id: String = input.interact_text()?;

        let mut config = HashMap::new();

        for prop in &node_type.properties {
            let value = self.prompt_property(prop)?;
            if !value.is_null() {
                config.insert(prop.name.clone(), value);
            }
        }

        // Convert HashMap<String, Value> to Value
        let config_value = serde_json::to_value(config)?;

        self.nodes.push(NodeDefinition {
            id,
            node_type: node_type.id.clone(),
            config: config_value,
        });

        println!("Node added!");
        Ok(())
    }

    fn prompt_property(&self, prop: &NodeProperty) -> Result<Value> {
        let theme = ColorfulTheme::default();
        match prop.property_type.as_str() {
            "text" | "code" | "json" => {
                loop {
                    let prompt = if prop.property_type == "json" {
                        format!("{} (JSON) [Type '?' for variables]", prop.label)
                    } else {
                        format!("{} [Type '?' for variables]", prop.label)
                    };

                    let mut input = Input::<String>::with_theme(&theme);
                    input = input.with_prompt(&prompt);
                    if let Some(def) = &prop.default {
                        input = input.default(def.clone());
                    }
                    
                    let val_str = input.interact_text()?;
                    
                    if val_str == "?" {
                        if let Some(var) = self.select_variable()? {
                            // If it's JSON, we might want to wrap it or just return as string if it's a template?
                            // If the property expects JSON, a string "{{...}}" is valid if the consumer handles it (which minijinja does).
                            // But `serde_json::from_str` will fail on "{{...}}".
                            // So we should return it as a String value, and the node logic must handle string-as-json-template.
                            // Our `AgentNode` handles this? No, it expects `Value`.
                            // Wait, if `property_type` is "json", we usually parse it.
                            // If we return a string, it might be rejected by the `json` parser below.
                            
                            // Let's handle the return type based on property type.
                            if prop.property_type == "json" {
                                // For JSON props, if it's a variable, we treat it as a String that *contains* the template.
                                // But `config` stores `Value`.
                                // We can store `Value::String("{{...}}")`.
                                // The node implementation must check if it's a string and try to parse/render.
                                return Ok(Value::String(var));
                            } else {
                                return Ok(Value::String(var));
                            }
                        }
                        continue;
                    }

                    if prop.property_type == "json" {
                         match serde_json::from_str(&val_str) {
                            Ok(v) => return Ok(v),
                            Err(_) => {
                                // Check if it looks like a template
                                if val_str.contains("{{") {
                                    return Ok(Value::String(val_str));
                                }
                                println!("Invalid JSON. If this is a template, ensure it is valid string.");
                                // Or just treat as string?
                                // Let's treat as string if it fails parsing, assuming it might be a template.
                                return Ok(Value::String(val_str));
                            }
                        }
                    } else {
                        return Ok(Value::String(val_str));
                    }
                }
            },
            "number" => {
                // Number fields usually don't support templates unless we change type to string in schema?
                // Or we allow string input that parses to number?
                // `Input::<f64>` won't accept "{{...}}".
                // We need to use `Input::<String>` and try to parse.
                loop {
                    let mut input = Input::<String>::with_theme(&theme);
                    input = input.with_prompt(&format!("{} (Number) [Type '?' for variables]", prop.label));
                    if let Some(def) = &prop.default {
                        input = input.default(def.clone());
                    }
                    
                    let val_str = input.interact_text()?;
                    if val_str == "?" {
                         if let Some(var) = self.select_variable()? {
                             // Return as string, but config expects number?
                             // If we store string in a number field, serde might fail later if struct expects f64.
                             // But `NodeDefinition.config` is `serde_json::Value`.
                             // So we can store a String. The node logic must handle it.
                             return Ok(Value::String(var));
                         }
                         continue;
                    }
                    
                    if let Ok(num) = val_str.parse::<f64>() {
                         if let Some(n) = serde_json::Number::from_f64(num) {
                             return Ok(Value::Number(n));
                         }
                    }
                    
                    // If not a number, is it a template?
                    if val_str.contains("{{") {
                        return Ok(Value::String(val_str));
                    }
                    
                    println!("Invalid number.");
                }
            },
            "boolean" => {
                let def = prop.default.as_deref() == Some("true");
                let val = Confirm::with_theme(&theme)
                    .with_prompt(&prop.label)
                    .default(def)
                    .interact()?;
                Ok(Value::Bool(val))
            },
            "select" => {
                if let Some(opts) = &prop.options {
                    let idx = Select::with_theme(&theme)
                        .with_prompt(&prop.label)
                        .items(opts)
                        .default(0)
                        .interact()?;
                    Ok(Value::String(opts[idx].clone()))
                } else {
                    Ok(Value::Null)
                }
            },
            _ => Ok(Value::Null),
        }
    }

    fn connect_nodes(&mut self) -> Result<()> {
        let theme = ColorfulTheme::default();
        if self.nodes.len() < 2 {
            println!("Need at least 2 nodes to connect.");
            return Ok(());
        }

        let node_ids: Vec<String> = self.nodes.iter().map(|n| n.id.clone()).collect();
        
        let from_idx = Select::with_theme(&theme)
            .with_prompt("From Node")
            .items(&node_ids)
            .interact()?;
        
        let to_idx = Select::with_theme(&theme)
            .with_prompt("To Node")
            .items(&node_ids)
            .interact()?;

        if from_idx == to_idx {
            println!("Cannot connect node to itself.");
            return Ok(());
        }

        let from_node = &self.nodes[from_idx];
        let registry = rust_flow::node_registry::get_node_registry();
        let from_node_type = registry.iter().find(|n| n.id == from_node.node_type).or_else(|| {
             // Fallback if ID doesn't match directly (e.g. integration nodes might have different ID format?)
             // Integration nodes have ID like "integration_node".
             registry.iter().find(|n| n.id == from_node.node_type)
        });

        let mut from_port = None;

        if let Some(nt) = from_node_type {
            if !nt.outputs.is_empty() {
                // If multiple outputs, ask user
                if nt.outputs.len() > 1 {
                    let selection = Select::with_theme(&theme)
                        .with_prompt("Select Output")
                        .items(&nt.outputs)
                        .default(0)
                        .interact()?;
                    from_port = Some(selection.to_string());
                } else {
                    // Only one output, use index 0
                    // But we don't strictly need to set it if it's 0, as default is 0.
                    // However, for clarity we can set it.
                    from_port = Some("0".to_string());
                }
            }
        }

        self.edges.push(EdgeDefinition {
            from: node_ids[from_idx].clone(),
            to: node_ids[to_idx].clone(),
            from_port,
            to_port: None,
        });

        println!("Connected!");
        Ok(())
    }

    fn list_nodes(&self) {
        println!("Nodes:");
        for node in &self.nodes {
            println!("- {} ({})", node.id, node.node_type);
        }
        println!("Edges:");
        for edge in &self.edges {
            println!("- {} -> {}", edge.from, edge.to);
        }
    }

    async fn test_node(&self) -> Result<()> {
        let theme = ColorfulTheme::default();
        if self.nodes.is_empty() {
            println!("No nodes to test.");
            return Ok(());
        }

        let node_ids: Vec<String> = self.nodes.iter().map(|n| n.id.clone()).collect();
        let idx = Select::with_theme(&theme)
            .with_prompt("Select Node to Test")
            .items(&node_ids)
            .interact()?;

        let target_node = &self.nodes[idx];
        
        // Build a mini-workflow with just this node (and potentially its parents if we want to be fancy, 
        // but for "Test Node" usually we want unit test style).
        // Actually, to test a node properly, we might need to supply input if it expects it.
        
        println!("Testing node '{}'...", target_node.id);
        
        // For MVP, we'll just run this single node.
        // If it requires input, we prompt for it.
        
        // Check if node has incoming edges
        let has_incoming = self.edges.iter().any(|e| e.to == target_node.id);
        
        let mut temp_nodes = vec![target_node.clone()];
        let mut temp_edges = vec![];

        if has_incoming {
            println!("Node has incoming edges. Do you want to provide mock input?");
            if Confirm::with_theme(&theme).with_prompt("Provide Mock Input?").interact()? {
                 let mut input = Input::with_theme(&theme);
                 input = input.with_prompt("Input JSON");
                 input = input.default("{}".to_string());
                 let input_str: String = input.interact_text()?;
                 
                 let input_json: Value = serde_json::from_str(&input_str).unwrap_or(serde_json::json!({}));
                 
                 // Create a dummy source node
                 let source_id = "mock_source";
                 temp_nodes.insert(0, NodeDefinition {
                     id: source_id.to_string(),
                     node_type: "set_data".to_string(),
                     config: serde_json::json!({ "json": input_json }),
                 });
                 
                 temp_edges.push(EdgeDefinition {
                     from: source_id.to_string(),
                     to: target_node.id.clone(),
                     from_port: None,
                     to_port: None,
                 });
            }
        } else {
             // If it's a trigger or source, we just run it.
             // But we need a sink to see output.
        }

        // Add a console output to see results
        let console_id = "debug_console";
        temp_nodes.push(NodeDefinition {
            id: console_id.to_string(),
            node_type: "console_output".to_string(),
            config: serde_json::json!({}),
        });
        
        temp_edges.push(EdgeDefinition {
            from: target_node.id.clone(),
            to: console_id.to_string(),
            from_port: None,
            to_port: None,
        });

        let workflow = WorkflowDefinition {
            nodes: temp_nodes,
            edges: temp_edges,
        };

        // Run it
        let secrets = HashMap::new();
        let debug_config = rust_flow::stream_engine::DebugConfig {
            limit_records: Some(1), // Safety limit
        };

        let mut executor = workflow.to_executor(&secrets, debug_config)?;
        
        // Setup event listener to show output and capture it
        let (tx, mut rx) = tokio::sync::broadcast::channel(100);
        executor.set_event_sender(tx);

        let (result_tx, mut result_rx) = tokio::sync::mpsc::channel(1);

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                 match event {
                     rust_flow::schema::ExecutionEvent::EdgeData { from: _, to, value } => {
                         if to == "debug_console" {
                             println!("Output: {}", serde_json::to_string_pretty(&value).unwrap_or_default());
                             let _ = result_tx.send(value).await;
                         }
                     },
                     rust_flow::schema::ExecutionEvent::NodeError { node_id, error } => {
                         eprintln!("Error in {}: {}", node_id, error);
                     },
                     _ => {}
                 }
            }
        });

        executor.run().await?;
        
        // Check if we captured output
        if let Ok(output) = result_rx.try_recv() {
            println!();
            if Confirm::with_theme(&theme).with_prompt("Inspect Output?").default(true).interact()? {
                crate::cli::inspector::run_inspector(&output)?;
            }
        }
        
        Ok(())
    }

    fn save_workflow(&self) -> Result<()> {
        let theme = ColorfulTheme::default();
        let mut input = Input::with_theme(&theme);
        input = input.with_prompt("Filename (.yaml)");
        input = input.default("workflow.yaml".to_string());
        let filename: String = input.interact_text()?;

        let workflow = WorkflowDefinition {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
        };

        let yaml = serde_yaml::to_string(&workflow)?;
        fs::write(&filename, yaml)?;
        println!("Saved to {}", filename);
        Ok(())
    }

    fn print_workflow_status(&self) {
        if self.nodes.is_empty() {
            println!("  (Empty Workflow)");
            return;
        }

        for node in &self.nodes {
            // Simple validation: check if config has all required props?
            // For now, let's assume if it's in the list, it's configured "enough" by the builder.
            // We can check if it's connected.
            
            let is_connected = self.edges.iter().any(|e| e.from == node.id || e.to == node.id);
            // Triggers don't need incoming, but need outgoing to be useful.
            // Sinks don't need outgoing.
            
            let status_icon = if is_connected || self.nodes.len() == 1 {
                style("✓").green()
            } else {
                style("!").yellow() // Warning: disconnected
            };
            
            println!("  {} {} ({})", status_icon, node.id, style(&node.node_type).dim());
            
            // Print outgoing edges
            for edge in self.edges.iter().filter(|e| e.from == node.id) {
                 println!("      └─> {}", edge.to);
            }
        }
    }

    async fn test_workflow(&self) -> Result<()> {
        if self.nodes.is_empty() {
            println!("No nodes to test.");
            return Ok(());
        }

        println!("Running workflow test...");
        
        let workflow = WorkflowDefinition {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
        };

        // Run it
        let secrets = HashMap::new();
        let debug_config = rust_flow::stream_engine::DebugConfig {
            limit_records: Some(5), 
        };

        let mut executor = workflow.to_executor(&secrets, debug_config)?;
        
        // Setup event listener to show output
        let (tx, mut rx) = tokio::sync::broadcast::channel(100);
        executor.set_event_sender(tx);

        // We want to capture the *last* significant output, or maybe all outputs?
        // For simplicity, let's capture the last EdgeData.
        let (result_tx, mut result_rx) = tokio::sync::mpsc::channel(100); // Buffer more

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                 match event {
                     rust_flow::schema::ExecutionEvent::NodeStart { node_id } => {
                         println!("{} Started: {}", style("•").blue(), node_id);
                     },
                     rust_flow::schema::ExecutionEvent::NodeFinish { node_id } => {
                         println!("{} Finished: {}", style("•").green(), node_id);
                     },
                     rust_flow::schema::ExecutionEvent::EdgeData { from, to, value } => {
                         println!("{} {} -> {}", style("→").cyan(), from, to);
                         println!("  Payload: {}", style(serde_json::to_string(&value).unwrap_or_default()).dim());
                         let _ = result_tx.send(value).await;
                     },
                     rust_flow::schema::ExecutionEvent::NodeError { node_id, error } => {
                         eprintln!("{} Error in {}: {}", style("x").red(), node_id, error);
                     },
                 }
            }
        });

        executor.run().await?;
        println!("{}", style("Test Complete").green().bold());
        
        // Offer inspection of the last output
        // Drain the channel to get the last one
        let mut last_val = None;
        while let Ok(val) = result_rx.try_recv() {
            last_val = Some(val);
        }

        if let Some(output) = last_val {
             if Confirm::with_theme(&ColorfulTheme::default()).with_prompt("Inspect Last Output?").default(true).interact()? {
                crate::cli::inspector::run_inspector(&output)?;
            }
        }
        
        Ok(())
    }

    fn select_variable(&self) -> Result<Option<String>> {
        if self.nodes.is_empty() {
            println!("{}", style("No previous nodes to reference.").yellow());
            return Ok(None);
        }

        let theme = ColorfulTheme::default();
        let mut node_choices: Vec<String> = self.nodes.iter().map(|n| format!("{} ({})", n.id, n.node_type)).collect();
        node_choices.push("Cancel".to_string());

        let selection = Select::with_theme(&theme)
            .with_prompt("Select Node")
            .items(&node_choices)
            .default(0)
            .interact()?;

        if selection == node_choices.len() - 1 {
            return Ok(None);
        }

        let selected_node = &self.nodes[selection];
        
        // Offer common paths based on node type
        let mut paths = vec![
            format!("Raw Output ({{{{ {} }}}})", selected_node.id),
        ];
        
        // Add common paths
        if selected_node.node_type.contains("Webhook") || selected_node.node_type.contains("Request") {
            paths.push(format!("Body ({{{{ {}.body }}}})", selected_node.id));
            paths.push(format!("Status ({{{{ {}.status }}}})", selected_node.id));
            paths.push(format!("Headers ({{{{ {}.headers }}}})", selected_node.id));
        } else if selected_node.node_type.contains("Agent") {
             paths.push(format!("Content ({{{{ {} }}}})", selected_node.id));
        }

        paths.push("Custom Path...".to_string());
        paths.push("Back".to_string());

        let path_sel = Select::with_theme(&theme)
            .with_prompt("Select Path")
            .items(&paths)
            .default(0)
            .interact()?;

        if path_sel == paths.len() - 1 {
            return self.select_variable();
        }

        if paths[path_sel] == "Custom Path..." {
             let input: String = Input::with_theme(&theme)
                .with_prompt("Enter Path (e.g. body.data[0].id)")
                .interact_text()?;
             return Ok(Some(format!("{{{{ {}.{} }}}}", selected_node.id, input)));
        }
        
        // Parse selection to get template
        // "Body ({{ node.id.body }})" -> "{{ node.id.body }}"
        let raw = &paths[path_sel];
        if let Some(start) = raw.find("{{") {
            if let Some(end) = raw.rfind("}}") {
                let template = &raw[start..end+2];
                return Ok(Some(template.to_string()));
            }
        }
        
        Ok(None)
    }
}
