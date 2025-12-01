use async_trait::async_trait;
use sqlx::{sqlite::SqlitePool, Row};
use uuid::Uuid;
use serde_json::Value;
use anyhow::Result;
use chrono::{Utc, DateTime};
use super::{Storage, Account, WorkflowEntity};

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn init(&self) -> Result<()> {
        // Create tables if not exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                name TEXT NOT NULL,
                definition TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(account_id) REFERENCES accounts(id)
            );
            CREATE TABLE IF NOT EXISTS key_value (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_account(&self, name: &str) -> Result<Account> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        
        sqlx::query(
            "INSERT INTO accounts (id, name, created_at) VALUES (?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(name)
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(Account {
            id,
            name: name.to_string(),
            created_at,
        })
    }

    async fn get_account(&self, id: Uuid) -> Result<Option<Account>> {
        let row = sqlx::query("SELECT id, name, created_at FROM accounts WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(Account {
                id: Uuid::parse_str(row.get("id"))?,
                name: row.get("name"),
                created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_account_by_name(&self, name: &str) -> Result<Option<Account>> {
        let row = sqlx::query("SELECT id, name, created_at FROM accounts WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(Account {
                id: Uuid::parse_str(row.get("id"))?,
                name: row.get("name"),
                created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
            }))
        } else {
            Ok(None)
        }
    }

    async fn save_workflow(&self, workflow: &WorkflowEntity) -> Result<()> {
        sqlx::query(
            "INSERT INTO workflows (id, account_id, name, definition, created_at) VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, definition=excluded.definition"
        )
        .bind(workflow.id.to_string())
        .bind(workflow.account_id.to_string())
        .bind(&workflow.name)
        .bind(serde_json::to_string(&workflow.definition)?)
        .bind(workflow.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowEntity>> {
        let row = sqlx::query("SELECT id, account_id, name, definition, created_at FROM workflows WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let def_str: String = row.get("definition");
            Ok(Some(WorkflowEntity {
                id: Uuid::parse_str(row.get("id"))?,
                account_id: Uuid::parse_str(row.get("account_id"))?,
                name: row.get("name"),
                definition: serde_json::from_str(&def_str)?,
                created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_workflows(&self, account_id: Uuid) -> Result<Vec<WorkflowEntity>> {
        let rows = sqlx::query("SELECT id, account_id, name, definition, created_at FROM workflows WHERE account_id = ?")
            .bind(account_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        let mut workflows = Vec::new();
        for row in rows {
            let def_str: String = row.get("definition");
            workflows.push(WorkflowEntity {
                id: Uuid::parse_str(row.get("id"))?,
                account_id: Uuid::parse_str(row.get("account_id"))?,
                name: row.get("name"),
                definition: serde_json::from_str(&def_str)?,
                created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
            });
        }
        Ok(workflows)
    }

    async fn set_kv(&self, key: &str, value: &Value) -> Result<()> {
        sqlx::query(
            "INSERT INTO key_value (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value"
        )
        .bind(key)
        .bind(serde_json::to_string(value)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_kv(&self, key: &str) -> Result<Option<Value>> {
        let row = sqlx::query("SELECT value FROM key_value WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let val_str: String = row.get("value");
            Ok(Some(serde_json::from_str(&val_str)?))
        } else {
            Ok(None)
        }
    }
}
