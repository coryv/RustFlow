use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use anyhow::Result;
use chrono::{DateTime, Utc};
use crate::schema::ExecutionEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: Uuid,
    pub workflow_id: Option<Uuid>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role {
    Admin,
    Member,
    Viewer,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Member => "member",
            Role::Viewer => "viewer",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Role::Admin),
            "member" => Some(Role::Member),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub role: Role,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEntity {
    pub id: Uuid,
    pub account_id: Uuid, // Keeping field name for now, but it refers to Team ID
    pub name: String,
    pub definition: Value, // Stored as JSON
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait Storage: Send + Sync {
    async fn init(&self) -> Result<()>;
    
    // User
    async fn create_user(&self, username: &str, password_hash: &str) -> Result<User>;
    async fn get_user(&self, id: Uuid) -> Result<Option<User>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>>;

    // Team
    async fn create_team(&self, name: &str) -> Result<Team>;
    async fn get_team(&self, id: Uuid) -> Result<Option<Team>>;
    async fn get_team_by_name(&self, name: &str) -> Result<Option<Team>>;
    async fn list_teams_for_user(&self, user_id: Uuid) -> Result<Vec<(Team, Role)>>;

    // Team Members
    async fn add_team_member(&self, team_id: Uuid, user_id: Uuid, role: Role) -> Result<()>;
    async fn get_team_members(&self, team_id: Uuid) -> Result<Vec<TeamMember>>;

    // Workflow
    async fn save_workflow(&self, workflow: &WorkflowEntity) -> Result<()>;
    async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowEntity>>;
    async fn list_workflows(&self, team_id: Uuid) -> Result<Vec<WorkflowEntity>>;

    // Key-Value (Cross-workflow state)
    async fn set_kv(&self, key: &str, value: &Value) -> Result<()>;
    async fn get_kv(&self, key: &str) -> Result<Option<Value>>;

    // Credentials
    async fn create_credential(&self, name: &str, credential_type: &str, data: &str, team_id: Uuid) -> Result<Credential>;
    async fn get_credential(&self, id: Uuid) -> Result<Option<Credential>>;
    async fn list_credentials(&self, team_id: Uuid) -> Result<Vec<Credential>>;


    // Executions
    async fn create_execution(&self, id: Uuid, workflow_id: Option<Uuid>, status: &str) -> Result<Uuid>;
    async fn update_execution(&self, id: Uuid, status: &str, finished_at: Option<DateTime<Utc>>, error: Option<String>) -> Result<()>;
    async fn log_execution_event(&self, execution_id: Uuid, event: &ExecutionEvent) -> Result<()>;
    async fn get_execution(&self, id: Uuid) -> Result<Option<ExecutionRecord>>;
    async fn get_execution_logs(&self, id: Uuid) -> Result<Vec<ExecutionEvent>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub credential_type: String,
    pub data: String, // Encrypted data (Base64 encoded)
    pub created_at: DateTime<Utc>,
}

pub mod sqlite;
pub mod encryption;
pub mod remote;
pub mod postgres;
pub use sqlite::SqliteStorage;
pub use remote::RemoteStorage;
pub use postgres::PostgresStorage;
