use async_trait::async_trait;
use sqlx::{postgres::PgPool, Row};
use uuid::Uuid;
use serde_json::Value;
use anyhow::{Result, anyhow};
use chrono::Utc;
use super::{Storage, Team, User, TeamMember, Role, WorkflowEntity, Credential, ExecutionRecord};
use crate::schema::ExecutionEvent;
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHasher, SaltString
    },
    Argon2
};

pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn init(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL
            );
            CREATE TABLE IF NOT EXISTS teams (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at TIMESTAMPTZ NOT NULL
            );
            CREATE TABLE IF NOT EXISTS team_members (
                user_id UUID NOT NULL,
                team_id UUID NOT NULL,
                role TEXT NOT NULL,
                joined_at TIMESTAMPTZ NOT NULL,
                PRIMARY KEY (user_id, team_id),
                FOREIGN KEY(user_id) REFERENCES users(id),
                FOREIGN KEY(team_id) REFERENCES teams(id)
            );
            CREATE TABLE IF NOT EXISTS workflows (
                id UUID PRIMARY KEY,
                account_id UUID NOT NULL,
                name TEXT NOT NULL,
                definition JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                FOREIGN KEY(account_id) REFERENCES teams(id)
            );
            CREATE TABLE IF NOT EXISTS key_value (
                key TEXT PRIMARY KEY,
                value JSONB NOT NULL
            );
            CREATE TABLE IF NOT EXISTS credentials (
                id UUID PRIMARY KEY,
                account_id UUID NOT NULL,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                FOREIGN KEY(account_id) REFERENCES teams(id)
            );
            CREATE TABLE IF NOT EXISTS executions (
                id UUID PRIMARY KEY,
                workflow_id UUID,
                status TEXT NOT NULL,
                started_at TIMESTAMPTZ NOT NULL,
                finished_at TIMESTAMPTZ,
                error TEXT,
                FOREIGN KEY(workflow_id) REFERENCES workflows(id)
            );
            CREATE TABLE IF NOT EXISTS execution_events (
                id UUID PRIMARY KEY,
                execution_id UUID NOT NULL,
                event_type TEXT NOT NULL,
                data JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                FOREIGN KEY(execution_id) REFERENCES executions(id)
            );
            "#
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // User
    async fn create_user(&self, username: &str, password: &str) -> Result<User> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Failed to hash password: {}", e))?
            .to_string();

        let id = Uuid::new_v4();
        let created_at = Utc::now();

        sqlx::query(
            "INSERT INTO users (id, username, password_hash, created_at) VALUES ($1, $2, $3, $4)"
        )
        .bind(id)
        .bind(username)
        .bind(&password_hash)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(User {
            id,
            username: username.to_string(),
            password_hash,
            created_at,
        })
    }

    async fn get_user(&self, id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query("SELECT id, username, password_hash, created_at FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(User {
                id: row.get("id"),
                username: row.get("username"),
                password_hash: row.get("password_hash"),
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query("SELECT id, username, password_hash, created_at FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(User {
                id: row.get("id"),
                username: row.get("username"),
                password_hash: row.get("password_hash"),
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    // Team
    async fn create_team(&self, name: &str) -> Result<Team> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        
        sqlx::query(
            "INSERT INTO teams (id, name, created_at) VALUES ($1, $2, $3)"
        )
        .bind(id)
        .bind(name)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(Team {
            id,
            name: name.to_string(),
            created_at,
        })
    }

    async fn get_team(&self, id: Uuid) -> Result<Option<Team>> {
        let row = sqlx::query("SELECT id, name, created_at FROM teams WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(Team {
                id: row.get("id"),
                name: row.get("name"),
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_team_by_name(&self, name: &str) -> Result<Option<Team>> {
        let row = sqlx::query("SELECT id, name, created_at FROM teams WHERE name = $1")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(Team {
                id: row.get("id"),
                name: row.get("name"),
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_teams_for_user(&self, user_id: Uuid) -> Result<Vec<(Team, Role)>> {
        let rows = sqlx::query(
            "SELECT t.id, t.name, t.created_at, tm.role 
             FROM teams t 
             JOIN team_members tm ON t.id = tm.team_id 
             WHERE tm.user_id = $1"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            let team = Team {
                id: row.get("id"),
                name: row.get("name"),
                created_at: row.get("created_at"),
            };
            let role_str: String = row.get("role");
            let role = Role::from_str(&role_str).ok_or_else(|| anyhow!("Invalid role string in DB"))?;
            results.push((team, role));
        }
        Ok(results)
    }

    // Team Members
    async fn add_team_member(&self, team_id: Uuid, user_id: Uuid, role: Role) -> Result<()> {
        let joined_at = Utc::now();
        sqlx::query(
            "INSERT INTO team_members (user_id, team_id, role, joined_at) VALUES ($1, $2, $3, $4)"
        )
        .bind(user_id)
        .bind(team_id)
        .bind(role.as_str())
        .bind(joined_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_team_members(&self, team_id: Uuid) -> Result<Vec<TeamMember>> {
        let rows = sqlx::query("SELECT user_id, team_id, role, joined_at FROM team_members WHERE team_id = $1")
            .bind(team_id)
            .fetch_all(&self.pool)
            .await?;

        let mut members = Vec::new();
        for row in rows {
            let role_str: String = row.get("role");
            members.push(TeamMember {
                user_id: row.get("user_id"),
                team_id: row.get("team_id"),
                role: Role::from_str(&role_str).ok_or_else(|| anyhow!("Invalid role string in DB"))?,
                joined_at: row.get("joined_at"),
            });
        }
        Ok(members)
    }

    // Workflow
    async fn save_workflow(&self, workflow: &WorkflowEntity) -> Result<()> {
        sqlx::query(
            "INSERT INTO workflows (id, account_id, name, definition, created_at) VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, definition=excluded.definition"
        )
        .bind(workflow.id)
        .bind(workflow.account_id)
        .bind(&workflow.name)
        .bind(&workflow.definition) // JSONB
        .bind(workflow.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowEntity>> {
        let row = sqlx::query("SELECT id, account_id, name, definition, created_at FROM workflows WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(WorkflowEntity {
                id: row.get("id"),
                account_id: row.get("account_id"),
                name: row.get("name"),
                definition: row.get("definition"), // JSONB
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_workflows(&self, team_id: Uuid) -> Result<Vec<WorkflowEntity>> {
        let rows = sqlx::query("SELECT id, account_id, name, definition, created_at FROM workflows WHERE account_id = $1")
            .bind(team_id)
            .fetch_all(&self.pool)
            .await?;

        let mut workflows = Vec::new();
        for row in rows {
            workflows.push(WorkflowEntity {
                id: row.get("id"),
                account_id: row.get("account_id"),
                name: row.get("name"),
                definition: row.get("definition"),
                created_at: row.get("created_at"),
            });
        }
        Ok(workflows)
    }

    // Key-Value
    async fn set_kv(&self, key: &str, value: &Value) -> Result<()> {
        sqlx::query(
            "INSERT INTO key_value (key, value) VALUES ($1, $2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value"
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_kv(&self, key: &str) -> Result<Option<Value>> {
        let row = sqlx::query("SELECT value FROM key_value WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(row.get("value")))
        } else {
            Ok(None)
        }
    }

    // Credentials
    async fn create_credential(&self, name: &str, credential_type: &str, data: &str, team_id: Uuid) -> Result<Credential> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let encrypted_data = super::encryption::encrypt(data)?;

        sqlx::query(
            "INSERT INTO credentials (id, account_id, name, type, data, created_at) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(id)
        .bind(team_id)
        .bind(name)
        .bind(credential_type)
        .bind(&encrypted_data)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(Credential {
            id,
            account_id: team_id,
            name: name.to_string(),
            credential_type: credential_type.to_string(),
            data: encrypted_data,
            created_at,
        })
    }

    async fn get_credential(&self, id: Uuid) -> Result<Option<Credential>> {
        let row = sqlx::query("SELECT id, account_id, name, type, data, created_at FROM credentials WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(Credential {
                id: row.get("id"),
                account_id: row.get("account_id"),
                name: row.get("name"),
                credential_type: row.get("type"),
                data: row.get("data"),
                created_at: row.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_credentials(&self, team_id: Uuid) -> Result<Vec<Credential>> {
        let rows = sqlx::query("SELECT id, account_id, name, type, data, created_at FROM credentials WHERE account_id = $1")
            .bind(team_id)
            .fetch_all(&self.pool)
            .await?;

        let mut creds = Vec::new();
        for row in rows {
            creds.push(Credential {
                id: row.get("id"),
                account_id: row.get("account_id"),
                name: row.get("name"),
                credential_type: row.get("type"),
                data: row.get("data"),
                created_at: row.get("created_at"),
            });
        }
        Ok(creds)
    }

    // Executions
    async fn create_execution(&self, id: Uuid, workflow_id: Option<Uuid>, status: &str) -> Result<Uuid> {
        let started_at = Utc::now();
        sqlx::query(
            "INSERT INTO executions (id, workflow_id, status, started_at) VALUES ($1, $2, $3, $4)"
        )
        .bind(id)
        .bind(workflow_id)
        .bind(status)
        .bind(started_at)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    async fn update_execution(&self, id: Uuid, status: &str, finished_at: Option<chrono::DateTime<Utc>>, error: Option<String>) -> Result<()> {
        sqlx::query(
            "UPDATE executions SET status = $1, finished_at = $2, error = $3 WHERE id = $4"
        )
        .bind(status)
        .bind(finished_at)
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn log_execution_event(&self, execution_id: Uuid, event: &ExecutionEvent) -> Result<()> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let event_type = match event {
            ExecutionEvent::NodeStart { .. } => "NodeStart",
            ExecutionEvent::NodeFinish { .. } => "NodeFinish",
            ExecutionEvent::NodeError { .. } => "NodeError",
            ExecutionEvent::EdgeData { .. } => "EdgeData",
            ExecutionEvent::WorkflowStart { .. } => "WorkflowStart",
            ExecutionEvent::WorkflowFinish { .. } => "WorkflowFinish",
        };
        let data = serde_json::to_value(event)?;
        
        sqlx::query(
            "INSERT INTO execution_events (id, execution_id, event_type, data, created_at) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(id)
        .bind(execution_id)
        .bind(event_type)
        .bind(data)
        .bind(created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_execution(&self, id: Uuid) -> Result<Option<ExecutionRecord>> {
        let row = sqlx::query("SELECT id, workflow_id, status, started_at, finished_at, error FROM executions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(ExecutionRecord {
                id: row.get("id"),
                workflow_id: row.get("workflow_id"),
                status: row.get("status"),
                started_at: row.get("started_at"),
                finished_at: row.get("finished_at"),
                error: row.get("error"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_execution_logs(&self, id: Uuid) -> Result<Vec<ExecutionEvent>> {
        let rows = sqlx::query("SELECT data FROM execution_events WHERE execution_id = $1 ORDER BY created_at ASC")
            .bind(id)
            .fetch_all(&self.pool)
            .await?;

        let mut events = Vec::new();
        for row in rows {
            let data: Value = row.get("data");
            let event: ExecutionEvent = serde_json::from_value(data)?;
            events.push(event);
        }
        Ok(events)
    }
}
