use axum::{
    routing::{post, get},
    Router,
    Json,
    response::IntoResponse,
    http::{StatusCode, Method},
    extract::{State, Path, Query, ws::{WebSocketUpgrade, WebSocket, Message}},
};
use tower_http::cors::{CorsLayer, Any};
use serde::{Deserialize, Serialize};
use rust_flow::schema::WorkflowLoader;
use rust_flow::job_manager::{JobManager, JobStatus};
use rust_flow::storage::{Storage, SqliteStorage, PostgresStorage, Role, WorkflowEntity};
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

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
    storage: Arc<dyn Storage>,
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
    
    // Initialize DB
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:rustflow.db".to_string());
    
    let storage: Arc<dyn Storage> = if db_url.starts_with("postgres://") {
        tracing::info!("Using PostgreSQL storage");
        let s = PostgresStorage::new(&db_url).await.expect("Failed to connect to Postgres");
        s.init().await.expect("Failed to init Postgres");
        Arc::new(s)
    } else {
        tracing::info!("Using SQLite storage");
        if db_url.starts_with("sqlite:") {
            let path = db_url.trim_start_matches("sqlite:");
            if !std::path::Path::new(path).exists() {
                std::fs::File::create(path).unwrap();
            }
        }
        let s = SqliteStorage::new(&db_url).await.expect("Failed to connect to SQLite");
        s.init().await.expect("Failed to init SQLite");
        Arc::new(s)
    };

    let state = Arc::new(AppState { job_manager, storage });

    let app = Router::new()
        .route("/api/run", post(run_workflow))
        .route("/api/jobs/{id}", get(get_job_status))
        .route("/api/ws/{id}", get(ws_handler))
        .route("/api/node-types", get(get_node_types))
        // Auth
        .route("/api/auth/register", post(register_user))
        .route("/api/auth/lookup", get(lookup_user))
        // Teams
        .route("/api/teams", post(create_team))
        .route("/api/users/{user_id}/teams", get(list_teams))
        .route("/api/teams/{team_id}/members", post(add_team_member).get(get_team_members))
        // Workflows
        .route("/api/workflows", post(save_workflow))
        .route("/api/teams/{team_id}/workflows", get(list_workflows))
        // Credentials
        .route("/api/credentials", post(create_credential))
        .route("/api/teams/{team_id}/credentials", get(list_credentials))
        
        .route("/health", get(|| async { "OK" }))
        .layer(cors)
        .with_state(state)
        .fallback_service(
            tower_http::services::ServeDir::new("dist")
                .not_found_service(tower_http::services::ServeFile::new("dist/index.html")),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- Handlers ---

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    password_hash: String,
}

async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>
) -> impl IntoResponse {
    match state.storage.create_user(&payload.username, &payload.password_hash).await {
        Ok(user) => (StatusCode::OK, Json(Some(user))),
        Err(_e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(None)),
    }
}

#[derive(Deserialize)]
struct LookupUserQuery {
    username: String,
}

async fn lookup_user(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LookupUserQuery>
) -> impl IntoResponse {
    match state.storage.get_user_by_username(&query.username).await {
        Ok(Some(user)) => (StatusCode::OK, Json(Some(user))),
        Ok(None) => (StatusCode::NOT_FOUND, Json(None)),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(None)),
    }
}

#[derive(Deserialize)]
struct CreateTeamReq {
    name: String,
}

async fn create_team(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateTeamReq>
) -> impl IntoResponse {
    match state.storage.create_team(&payload.name).await {
        Ok(team) => (StatusCode::OK, Json(Some(team))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(None)),
    }
}

async fn list_teams(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>
) -> impl IntoResponse {
    match state.storage.list_teams_for_user(user_id).await {
        Ok(teams) => (StatusCode::OK, Json(Some(teams))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(None)),
    }
}

#[derive(Deserialize)]
struct AddMemberReq {
    user_id: Uuid,
    role: String,
}

async fn add_team_member(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>,
    Json(payload): Json<AddMemberReq>
) -> impl IntoResponse {
    let role = match Role::from_str(&payload.role) {
        Some(r) => r,
        None => return StatusCode::BAD_REQUEST,
    };
    
    match state.storage.add_team_member(team_id, payload.user_id, role).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn get_team_members(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>
) -> impl IntoResponse {
    match state.storage.get_team_members(team_id).await {
        Ok(members) => (StatusCode::OK, Json(Some(members))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(None)),
    }
}

async fn save_workflow(
    State(state): State<Arc<AppState>>,
    Json(workflow): Json<WorkflowEntity>
) -> impl IntoResponse {
    match state.storage.save_workflow(&workflow).await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            tracing::error!("Failed to save workflow: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        },
    }
}

async fn list_workflows(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>
) -> impl IntoResponse {
    match state.storage.list_workflows(team_id).await {
        Ok(wfs) => (StatusCode::OK, Json(Some(wfs))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(None)),
    }
}

#[derive(Deserialize)]
struct CreateCredReq {
    name: String,
    credential_type: String,
    data: String,
    team_id: Uuid,
}

async fn create_credential(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateCredReq>
) -> impl IntoResponse {
    match state.storage.create_credential(&payload.name, &payload.credential_type, &payload.data, payload.team_id).await {
        Ok(cred) => (StatusCode::OK, Json(Some(cred))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(None)),
    }
}

async fn list_credentials(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<Uuid>
) -> impl IntoResponse {
    match state.storage.list_credentials(team_id).await {
        Ok(creds) => (StatusCode::OK, Json(Some(creds))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(None)),
    }
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

    let executor = match workflow_def.to_executor(&secrets, rust_flow::stream_engine::DebugConfig::default()) {
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
