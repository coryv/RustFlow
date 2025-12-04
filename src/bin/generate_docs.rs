use rust_flow::node_registry::get_node_registry;

fn main() {
    let nodes = get_node_registry();

    println!("# RustFlow Node Reference\n");
    println!("This document is auto-generated. Do not edit manually.\n");

    // Group by category
    let mut nodes_by_category: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
    for node in &nodes {
        nodes_by_category.entry(node.category.clone()).or_default().push(node);
    }

    let mut categories: Vec<_> = nodes_by_category.keys().cloned().collect();
    categories.sort();

    for category in categories {
        println!("## {}\n", category);
        
        let mut category_nodes = nodes_by_category.get(&category).unwrap().clone();
        category_nodes.sort_by_key(|n| n.label.clone());

        for node in category_nodes {
            println!("### {}\n", node.label);
            println!("**ID**: `{}`\n", node.id);
            
            if let Some(desc) = &node.description {
                println!("{}\n", desc);
            }

            if let Some(docs) = &node.documentation {
                println!("{}\n", docs.trim());
            }

            if !node.properties.is_empty() {
                println!("#### Properties\n");
                println!("| Name | Type | Required | Default | Description |");
                println!("|------|------|----------|---------|-------------|");
                
                for prop in &node.properties {
                    let required = if prop.required { "Yes" } else { "No" };
                    let default = prop.default.as_deref().unwrap_or("-");
                    println!("| `{}` | `{}` | {} | `{}` | {} |", 
                        prop.name, 
                        prop.property_type, 
                        required, 
                        default, 
                        prop.label
                    );
                }
                println!();
            }

            if !node.outputs.is_empty() {
                println!("#### Outputs (Named Ports)\n");
                println!("| Port Name | Index |");
                println!("|-----------|-------|");
                for (i, output) in node.outputs.iter().enumerate() {
                    println!("| `{}` | `{}` |", output, i);
                }
                println!();
            }
            println!("---\n");
        }
    }
}
