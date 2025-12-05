use rust_flow::schema::{WorkflowDefinition, NodeDefinition, EdgeDefinition};
use rust_flow::stream_engine::DebugConfig;
use serde_json::{json, Value};
use std::collections::HashMap;

#[tokio::test]
async fn test_select_node_functionality() {
    // 1. Setup Logic: Source -> Select -> Sink
    let nodes = vec![
        NodeDefinition {
            id: "source".to_string(),
            node_type: "set_data".to_string(),
            config: json!({ "json": { "user": { "id": 123, "name": "Alice" } } }),
            on_error: None,
        },
        NodeDefinition {
            id: "select_transformer".to_string(),
            node_type: "select".to_string(),
            config: json!({ 
                "template": "{ \"uid\": {{ json.user.id }}, \"greeting\": \"Hello {{ json.user.name }}\" }" 
            }),
            on_error: None,
        },
        NodeDefinition {
            id: "sink".to_string(),
            node_type: "console_output".to_string(),
            config: json!({}),
            on_error: None,
        },
    ];

    let edges = vec![
        EdgeDefinition {
            from: "source".to_string(),
            to: "select_transformer".to_string(),
            from_port: None,
            to_port: None,
        },
        EdgeDefinition {
            from: "select_transformer".to_string(),
            to: "sink".to_string(),
            from_port: None,
            to_port: None,
        },
    ];

    let workflow = WorkflowDefinition { nodes, edges };
    
    let mut executor = workflow.to_executor(&HashMap::new(), DebugConfig::default()).expect("Failed to create executor");
    executor.inject_input("source", json!(null)); // Trigger the set_data node
    
    let (tx, mut rx) = tokio::sync::broadcast::channel(10);
    executor.set_event_sender(tx);
    
    // Spawn executor
    let handle = tokio::spawn(async move {
        executor.run().await
    });
    
    // Listen for events
    let mut found = false;
    while let Ok(event) = rx.recv().await {
        println!("Event: {:?}", event);
        match event {
            rust_flow::schema::ExecutionEvent::EdgeData { from, to, value } => {
                if from == "select_transformer" && to == "sink" {
                    println!("Captured Output: {:?}", value);
                    assert_eq!(value["uid"], 123);
                    assert_eq!(value["greeting"], "Hello Alice");
                    found = true;
                    break;
                }
            },
            rust_flow::schema::ExecutionEvent::NodeError { node_id, error } => {
                println!("Error in {}: {}", node_id, error);
            },
            _ => {}
        }
    }
    
    assert!(found, "Did not receive expected transformed data");
    
    // Wait for finish
    handle.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_select_node_casting() {
    // Test Type Casting
    // Source -> Select (Cast) -> Sink

    let nodes = vec![
        NodeDefinition {
            id: "source".to_string(),
            node_type: "set_data".to_string(),
            config: json!({ "json": { "val": "123.45", "bool_str": "true", "json_str": "{\"a\":1}" } }),
            on_error: None,
        },
        NodeDefinition {
            id: "cast_number".to_string(),
            node_type: "select".to_string(),
            config: json!({ 
                "template": "{{ json.val }}",
                "output_type": "number"
            }),
            on_error: None,
        },
        NodeDefinition {
            id: "cast_bool".to_string(),
            node_type: "select".to_string(),
            config: json!({ 
                "template": "{{ json.bool_str }}",
                "output_type": "boolean"
            }),
            on_error: None,
        },
        NodeDefinition {
            id: "sink".to_string(),
            node_type: "console_output".to_string(),
            config: json!({}),
            on_error: None,
        },
    ];

    let edges = vec![
        EdgeDefinition { from: "source".to_string(), to: "cast_number".to_string(), from_port: None, to_port: None },
        EdgeDefinition { from: "source".to_string(), to: "cast_bool".to_string(), from_port: None, to_port: None },
        EdgeDefinition { from: "cast_number".to_string(), to: "sink".to_string(), from_port: None, to_port: None },
        EdgeDefinition { from: "cast_bool".to_string(), to: "sink".to_string(), from_port: None, to_port: None },
    ];

    let workflow = WorkflowDefinition { nodes, edges };
    let mut executor = workflow.to_executor(&HashMap::new(), DebugConfig::default()).expect("Failed to create executor");
    executor.inject_input("source", json!(null));
    
    let (tx, mut rx) = tokio::sync::broadcast::channel(10);
    executor.set_event_sender(tx);
    
    tokio::spawn(async move {
        executor.run().await
    });
    
    let mut num_found = false;
    let mut bool_found = false;

    while let Ok(event) = rx.recv().await {
        if let rust_flow::schema::ExecutionEvent::EdgeData { from, to: _, value } = event {
            if from == "cast_number" {
                println!("Captured Number: {:?}", value);
                assert!(value.is_number());
                assert_eq!(value.as_f64().unwrap(), 123.45);
                num_found = true;
            }
            if from == "cast_bool" {
                println!("Captured Bool: {:?}", value);
                assert!(value.is_boolean());
                assert_eq!(value.as_bool().unwrap(), true);
                bool_found = true;
            }
        }
        if num_found && bool_found { break; }
    }
    
    assert!(num_found, "Number cast failed");
    assert!(bool_found, "Boolean cast failed");
}
