use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select, Input};
use console::style;
use serde_json::Value;

pub fn run_inspector(root_data: &Value) -> Result<()> {
    let mut stack: Vec<(String, &Value)> = vec![("$".to_string(), root_data)];
    
    loop {
        let (_name, current_val) = stack.last().unwrap();
        
        // Build full path
        let path_str = stack.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(".");
        // Fix up array syntax: $.foo.[0] -> $.foo[0]
        let display_path = path_str.replace(".[", "[");
        
        println!("\n{}", style(format!("Path: {}", display_path)).cyan().bold());
        
        // Preview
        match current_val {
            Value::Object(map) => {
                println!("Type: Object ({} keys)", map.len());
                for (k, v) in map.iter().take(10) {
                     let preview = match v {
                         Value::String(_) => "String",
                         Value::Number(_) => "Number",
                         Value::Bool(_) => "Bool",
                         Value::Null => "Null",
                         Value::Array(_) => "Array",
                         Value::Object(_) => "Object",
                     };
                     println!("  {}: {}", style(k).blue(), style(preview).dim());
                }
                if map.len() > 10 { println!("  ..."); }
            },
            Value::Array(arr) => {
                println!("Type: Array ({} items)", arr.len());
                for (i, _v) in arr.iter().enumerate().take(5) {
                    println!("  [{}]: ...", i);
                }
            },
            val => {
                println!("Value: {}", serde_json::to_string_pretty(val).unwrap());
            }
        }

        let mut choices = vec![];
        match current_val {
            Value::Object(map) => {
                let mut keys: Vec<_> = map.keys().collect();
                keys.sort();
                for key in keys {
                    choices.push(format!("Dive: .{}", key));
                }
            },
            Value::Array(arr) => {
                for i in 0..arr.len() {
                    choices.push(format!("Dive: [{}]", i));
                }
            },
            _ => {}
        }
        
        choices.push("Query (JMESPath)".to_string());
        if stack.len() > 1 {
            choices.push("Back".to_string());
        }
        choices.push("Exit".to_string());

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Inspect")
            .default(0)
            .items(&choices)
            .interact()?;

        let choice = &choices[selection];

        if choice.starts_with("Dive: .") {
            let key = choice.trim_start_matches("Dive: .");
            if let Value::Object(map) = current_val {
                if let Some(v) = map.get(key) {
                    stack.push((format!(".{}", key), v));
                }
            }
        } else if choice.starts_with("Dive: [") {
            let idx_str = choice.trim_start_matches("Dive: [").trim_end_matches(']');
            if let Ok(idx) = idx_str.parse::<usize>() {
                if let Value::Array(arr) = current_val {
                    if let Some(v) = arr.get(idx) {
                        stack.push((format!("[{}]", idx), v));
                    }
                }
            }
        } else if choice == "Query (JMESPath)" {
             run_jmespath_query(root_data)?;
        } else if choice == "Back" {
            stack.pop();
        } else if choice == "Exit" {
            break;
        }
    }
    Ok(())
}

fn run_jmespath_query(data: &Value) -> Result<()> {
    let theme = ColorfulTheme::default();
    println!("\n{}", style("--- JMESPath Query Mode ---").magenta());
    println!("Enter a query to filter data. Empty to exit.");
    
    loop {
        let input: String = Input::with_theme(&theme)
            .with_prompt("Query")
            .allow_empty(true)
            .interact_text()?;
        
        if input.is_empty() {
            break;
        }

        match jmespath::compile(&input) {
            Ok(expr) => {
                match expr.search(data) {
                    Ok(result) => {
                        println!("{}", serde_json::to_string_pretty(&*result).unwrap());
                    },
                    Err(e) => println!("{}", style(format!("Runtime Error: {}", e)).red()),
                }
            },
            Err(e) => println!("{}", style(format!("Invalid Query: {}", e)).red()),
        }
    }
    Ok(())
}
