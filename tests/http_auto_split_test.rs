use rust_flow::schema::{WorkflowDefinition, NodeDefinition, EdgeDefinition};
use rust_flow::stream_engine::DebugConfig;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[tokio::test]
async fn test_http_auto_split() {
    // Spin up a simple HTTP server on a random port
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{}/test", port);

    thread::spawn(move || {
        for stream in listener.incoming() {
             let mut stream = stream.unwrap();
             let mut buffer = [0; 1024];
             stream.read(&mut buffer).unwrap();
             
             let body = r#"[{"id":1},{"id":2}]"#;
             let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
             stream.write_all(response.as_bytes()).unwrap();
             stream.flush().unwrap();
        }
    });

    // Workflow: Manual -> HTTP Request (Auto-Split) -> Sink
    let nodes = vec![
        NodeDefinition {
            id: "source".to_string(),
            node_type: "manual_trigger".to_string(),
            config: json!({}),
            on_error: None,
        },
        NodeDefinition {
            id: "http".to_string(),
            node_type: "http_request".to_string(),
            config: json!({ 
                "method": "GET",
                "url": url,
                "auto_split": true 
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
        EdgeDefinition { from: "source".to_string(), to: "http".to_string(), from_port: None, to_port: None },
        EdgeDefinition { from: "http".to_string(), to: "sink".to_string(), from_port: None, to_port: None },
    ];

    let workflow = WorkflowDefinition { nodes, edges };
    let mut executor = workflow.to_executor(&HashMap::new(), DebugConfig::default()).expect("Failed to create executor");
    executor.inject_input("source", json!(null));
    
    let (tx, mut rx) = tokio::sync::broadcast::channel(100);
    executor.set_event_sender(tx);
    
    tokio::spawn(async move {
        executor.run().await
    });
    
    let mut received_count = 0;
    while let Ok(event) = rx.recv().await {
        if let rust_flow::schema::ExecutionEvent::EdgeData { from, to: _, value } = event {
            if from == "http" {
                println!("Captured Item: {:?}", value);
                // Schema: { "body": item, ... }
                let body = value.get("body").unwrap();
                let id = body.get("id").unwrap().as_i64().unwrap();
                assert!(id == 1 || id == 2);
                received_count += 1;
            }
        }
    }
    
    // Note: The loop might exit before processing all if we don't wait correctly, 
    // but run() finishes when processing is done.
    // However, ManualTrigger emits once. HTTP requests once. Emits 2 items.
    // The executor should close naturally.
    
    assert_eq!(received_count, 2, "Expected 2 items from auto-split");
}
