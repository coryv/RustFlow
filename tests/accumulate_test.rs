use rust_flow::schema::{WorkflowDefinition, NodeDefinition, EdgeDefinition};
use rust_flow::stream_engine::DebugConfig;
use serde_json::{json, Value};
use std::collections::HashMap;

#[tokio::test]
async fn test_accumulate_all() {
    // SetData([1..5]) -> Split -> Accumulate(None) -> Sink
    // Should get [1,2,3,4,5] once.

    let nodes = vec![
        NodeDefinition {
            id: "source".to_string(),
            node_type: "set_data".to_string(),
            config: json!({ "json": [1, 2, 3, 4, 5] }),
            on_error: None,
        },
        NodeDefinition {
            id: "split".to_string(),
            node_type: "split".to_string(),
            config: json!({ "path": "json" }),
            on_error: None,
        },
        NodeDefinition {
            id: "accumulate".to_string(),
            node_type: "accumulate".to_string(),
            config: json!({}),
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
        EdgeDefinition { from: "source".to_string(), to: "split".to_string(), from_port: None, to_port: None },
        EdgeDefinition { from: "split".to_string(), to: "accumulate".to_string(), from_port: None, to_port: None },
        EdgeDefinition { from: "accumulate".to_string(), to: "sink".to_string(), from_port: None, to_port: None },
    ];

    let workflow = WorkflowDefinition { nodes, edges };
    let mut executor = workflow.to_executor(&HashMap::new(), DebugConfig::default()).expect("Failed to create executor");
    executor.inject_input("source", json!(null));
    
    let (tx, mut rx) = tokio::sync::broadcast::channel(100);
    executor.set_event_sender(tx);
    
    tokio::spawn(async move {
        executor.run().await
    });
    
    let mut found = false;
    while let Ok(event) = rx.recv().await {
        if let rust_flow::schema::ExecutionEvent::EdgeData { from, to: _, value } = event {
            if from == "accumulate" {
                println!("Captured Accumulate All: {:?}", value);
                assert!(value.is_array());
                let arr = value.as_array().unwrap();
                assert_eq!(arr.len(), 5);
                found = true;
            }
        }
    }
    assert!(found, "Did not receive accumulated output");
}

#[tokio::test]
async fn test_accumulate_batch() {
    // SetData([1..13]) -> Split -> Accumulate(5) -> Sink
    // Should get [5], [5], [3]

    let input_arr: Vec<i32> = (1..=13).collect();
    
    let nodes = vec![
        NodeDefinition {
            id: "source".to_string(),
            node_type: "set_data".to_string(),
            config: json!({ "json": input_arr }),
            on_error: None,
        },
        NodeDefinition {
            id: "split".to_string(),
            node_type: "split".to_string(),
            config: json!({ "path": "json" }),
            on_error: None,
        },
        NodeDefinition {
            id: "accumulate".to_string(),
            node_type: "accumulate".to_string(),
            config: json!({ "batch_size": 5 }),
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
        EdgeDefinition { from: "source".to_string(), to: "split".to_string(), from_port: None, to_port: None },
        EdgeDefinition { from: "split".to_string(), to: "accumulate".to_string(), from_port: None, to_port: None },
        EdgeDefinition { from: "accumulate".to_string(), to: "sink".to_string(), from_port: None, to_port: None },
    ];

    let workflow = WorkflowDefinition { nodes, edges };
    let mut executor = workflow.to_executor(&HashMap::new(), DebugConfig::default()).expect("Failed to create executor");
    executor.inject_input("source", json!(null));
    
    let (tx, mut rx) = tokio::sync::broadcast::channel(100);
    executor.set_event_sender(tx);
    
    tokio::spawn(async move {
        executor.run().await
    });
    
    let mut batches = Vec::new();
    while let Ok(event) = rx.recv().await {
        if let rust_flow::schema::ExecutionEvent::EdgeData { from, to: _, value } = event {
            if from == "accumulate" {
                println!("Captured Batch: {:?}", value);
                batches.push(value);
            }
        }
    }
    
    assert_eq!(batches.len(), 3, "Expected 3 batches");
    assert_eq!(batches[0].as_array().unwrap().len(), 5);
    assert_eq!(batches[1].as_array().unwrap().len(), 5);
    assert_eq!(batches[2].as_array().unwrap().len(), 3);
}
