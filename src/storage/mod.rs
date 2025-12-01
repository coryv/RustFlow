use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use anyhow::Result;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEntity {
    pub id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub definition: Value, // Stored as JSON
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait Storage: Send + Sync {
    async fn init(&self) -> Result<()>;
    
    // Account
    async fn create_account(&self, name: &str) -> Result<Account>;
    async fn get_account(&self, id: Uuid) -> Result<Option<Account>>;
    async fn get_account_by_name(&self, name: &str) -> Result<Option<Account>>;

    // Workflow
    async fn save_workflow(&self, workflow: &WorkflowEntity) -> Result<()>;
    async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowEntity>>;
    async fn list_workflows(&self, account_id: Uuid) -> Result<Vec<WorkflowEntity>>;

    // Key-Value (Cross-workflow state)
    async fn set_kv(&self, key: &str, value: &Value) -> Result<()>;
    async fn get_kv(&self, key: &str) -> Result<Option<Value>>;
}

pub mod sqlite;
pub use sqlite::SqliteStorage;
