use axum::{
    routing::{post, get},
    Router,
    Json,
    response::IntoResponse,
    http::{StatusCode, Method},
    extract::{State, Path, ws::{WebSocketUpgrade, WebSocket, Message}},
};
use tower_http::cors::{CorsLayer, Any};
use serde::{Deserialize, Serialize};
use rust_flow::schema::WorkflowLoader;
use rust_flow::job_manager::{JobManager, JobStatus};
use std::net::SocketAddr;
use std::sync::Arc;
use futures::sink::SinkExt;

#[derive(Deserialize)]
struct RunRequest {
    workflow: String, // YAML or JSON string
}

#[derive(Serialize)]
struct RunResponse {
    job_id: Option<String>,
    status: String,
    error: Option<String>,
}

#[derive(Serialize)]
struct JobStatusResponse {
    id: String,
    status: String,
    logs: Vec<String>,
}

struct AppState {
    job_manager: Arc<JobManager>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let job_manager = Arc::new(JobManager::new());
    let state = Arc::new(AppState { job_manager });

    let app = Router::new()
        .route("/api/run", post(run_workflow))
        .route("/api/jobs/{id}", get(get_job_status))
        .route("/api/ws/{id}", get(ws_handler))
        .route("/api/node-types", get(get_node_types))
        .route("/health", get(|| async { "OK" }))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn run_workflow(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RunRequest>
) -> impl IntoResponse {
    tracing::info!("Received workflow execution request");
    
    let loader = WorkflowLoader::new();
    let workflow_def = match loader.load(&payload.workflow) {
        Ok(def) => def,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(RunResponse {
            job_id: None,
            status: "error".to_string(),
            error: Some(format!("Failed to parse workflow: {}", e)),
        })),
    };

    // TODO: Load credentials from DB if needed. For now, empty.
    let secrets = std::collections::HashMap::new();

    let executor = match workflow_def.to_executor(&secrets) {
        Ok(ex) => ex,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(RunResponse {
            job_id: None,
            status: "error".to_string(),
            error: Some(format!("Failed to build executor: {}", e)),
        })),
    };

    // Create job and spawn execution
    let job_id = state.job_manager.create_job();
    let manager = state.job_manager.clone();
    let id_clone = job_id.clone();

    tokio::spawn(async move {
        manager.run_job(id_clone, executor).await;
    });

    (StatusCode::OK, Json(RunResponse {
        job_id: Some(job_id),
        status: "pending".to_string(),
        error: None,
    }))
}

async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>
) -> impl IntoResponse {
    if let Some(status) = state.job_manager.get_job(&id) {
        let logs = state.job_manager.get_job_logs(&id).unwrap_or_default();
        let status_str = match status {
            JobStatus::Pending => "pending",
            JobStatus::Running => "running",
            JobStatus::Completed => "completed",
            JobStatus::Failed(_) => "failed",
        };

        (StatusCode::OK, Json(JobStatusResponse {
            id,
            status: status_str.to_string(),
            logs,
        }))
    } else {
        (StatusCode::NOT_FOUND, Json(JobStatusResponse {
            id,
            status: "not_found".to_string(),
            logs: vec![],
        }))
    }
}

async fn get_node_types() -> impl IntoResponse {
    let registry = rust_flow::node_registry::get_node_registry();
    Json(registry)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, id, state))
}

async fn handle_socket(mut socket: WebSocket, job_id: String, state: Arc<AppState>) {
    let mut rx = match state.job_manager.subscribe_to_events(&job_id) {
        Some(rx) => rx,
        None => {
            let _ = socket.send(Message::Text("Job not found".into())).await;
            return;
        }
    };

    while let Ok(event) = rx.recv().await {
        if let Ok(msg) = serde_json::to_string(&event) {
            if socket.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    }
}
