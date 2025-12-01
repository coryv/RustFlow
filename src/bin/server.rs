use axum::{
    routing::{post, get},
    Router,
    Json,
    response::IntoResponse,
    http::{StatusCode, Method},
};
use tower_http::cors::{CorsLayer, Any};
use serde::{Deserialize, Serialize};
use rust_flow::schema::WorkflowLoader;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

#[derive(Deserialize)]
struct RunRequest {
    workflow: String, // YAML or JSON string
}

#[derive(Serialize)]
struct RunResponse {
    status: String,
    logs: Vec<String>,
    error: Option<String>,
}

// Simple in-memory log capture (for demo purposes)
// In a real app, we'd use a more sophisticated logging/tracing setup
struct LogCapture {
    logs: Arc<Mutex<Vec<String>>>,
}

impl LogCapture {
    fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/run", post(run_workflow))
        .route("/api/node-types", get(get_node_types))
        .route("/health", get(|| async { "OK" }))
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn run_workflow(Json(payload): Json<RunRequest>) -> impl IntoResponse {
    println!("Received workflow execution request");
    
    let loader = WorkflowLoader::new();
    let workflow_def = match loader.load(&payload.workflow) {
        Ok(def) => def,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(RunResponse {
            status: "error".to_string(),
            logs: vec![],
            error: Some(format!("Failed to parse workflow: {}", e)),
        })),
    };

    // TODO: Load credentials from DB if needed. For now, empty.
    let secrets = std::collections::HashMap::new();

    let executor = match workflow_def.to_executor(&secrets) {
        Ok(ex) => ex,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(RunResponse {
            status: "error".to_string(),
            logs: vec![],
            error: Some(format!("Failed to build executor: {}", e)),
        })),
    };

    // Run the workflow
    // Note: This runs synchronously in the handler for simplicity.
    // For long-running workflows, we should spawn a task and return a job ID.
    match executor.run().await {
        Ok(_) => {
            (StatusCode::OK, Json(RunResponse {
                status: "success".to_string(),
                logs: vec!["Workflow executed successfully".to_string()], // Placeholder for real logs
                error: None,
            }))
        },
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(RunResponse {
                status: "error".to_string(),
                logs: vec![],
                error: Some(format!("Execution failed: {}", e)),
            }))
        }
    }
}

async fn get_node_types() -> impl IntoResponse {
    let registry = rust_flow::node_registry::get_node_registry();
    Json(registry)
}
