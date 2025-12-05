use async_trait::async_trait;
use sqlx::{sqlite::SqlitePool, Row};
use uuid::Uuid;
use serde_json::Value;
use anyhow::{Result, anyhow};
use chrono::{Utc, DateTime};
use super::{Storage, Team, User, TeamMember, Role, WorkflowEntity, Credential, ExecutionRecord};
use crate::schema::ExecutionEvent;
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHasher, SaltString
    },
    Argon2
};

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
        // Note: 'accounts' is renamed to 'teams' conceptually, but we'll use 'teams' table name.
        // If 'accounts' exists, we might want to migrate, but for now we'll just create new tables.
        // In a real scenario, we'd have a migration script.
        
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS teams (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS team_members (
                user_id TEXT NOT NULL,
                team_id TEXT NOT NULL,
                role TEXT NOT NULL,
                joined_at TEXT NOT NULL,
                PRIMARY KEY (user_id, team_id),
                FOREIGN KEY(user_id) REFERENCES users(id),
                FOREIGN KEY(team_id) REFERENCES teams(id)
            );
            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL, -- This is team_id
                name TEXT NOT NULL,
                definition TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(account_id) REFERENCES teams(id)
            );
            CREATE TABLE IF NOT EXISTS key_value (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS credentials (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL, -- This is team_id
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(account_id) REFERENCES teams(id)
            );
            CREATE TABLE IF NOT EXISTS executions (
                id TEXT PRIMARY KEY,
                workflow_id TEXT, -- Nullable
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                error TEXT,
                FOREIGN KEY(workflow_id) REFERENCES workflows(id)
            );
            CREATE TABLE IF NOT EXISTS execution_events (
                id TEXT PRIMARY KEY,
                execution_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL,
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
            "INSERT INTO users (id, username, password_hash, created_at) VALUES (?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(username)
        .bind(&password_hash)
        .bind(created_at.to_rfc3339())
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
        let row = sqlx::query("SELECT id, username, password_hash, created_at FROM users WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(User {
                id: Uuid::parse_str(row.get("id"))?,
                username: row.get("username"),
                password_hash: row.get("password_hash"),
                created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query("SELECT id, username, password_hash, created_at FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(User {
                id: Uuid::parse_str(row.get("id"))?,
                username: row.get("username"),
                password_hash: row.get("password_hash"),
                created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
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
            "INSERT INTO teams (id, name, created_at) VALUES (?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(name)
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(Team {
            id,
            name: name.to_string(),
            created_at,
        })
    }

    async fn get_team(&self, id: Uuid) -> Result<Option<Team>> {
        let row = sqlx::query("SELECT id, name, created_at FROM teams WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(Team {
                id: Uuid::parse_str(row.get("id"))?,
                name: row.get("name"),
                created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_team_by_name(&self, name: &str) -> Result<Option<Team>> {
        let row = sqlx::query("SELECT id, name, created_at FROM teams WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(Team {
                id: Uuid::parse_str(row.get("id"))?,
                name: row.get("name"),
                created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
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
             WHERE tm.user_id = ?"
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            let team = Team {
                id: Uuid::parse_str(row.get("id"))?,
                name: row.get("name"),
                created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
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
            "INSERT INTO team_members (user_id, team_id, role, joined_at) VALUES (?, ?, ?, ?)"
        )
        .bind(user_id.to_string())
        .bind(team_id.to_string())
        .bind(role.as_str())
        .bind(joined_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_team_members(&self, team_id: Uuid) -> Result<Vec<TeamMember>> {
        let rows = sqlx::query("SELECT user_id, team_id, role, joined_at FROM team_members WHERE team_id = ?")
            .bind(team_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        let mut members = Vec::new();
        for row in rows {
            let role_str: String = row.get("role");
            members.push(TeamMember {
                user_id: Uuid::parse_str(row.get("user_id"))?,
                team_id: Uuid::parse_str(row.get("team_id"))?,
                role: Role::from_str(&role_str).ok_or_else(|| anyhow!("Invalid role string in DB"))?,
                joined_at: DateTime::parse_from_rfc3339(row.get("joined_at"))?.with_timezone(&Utc),
            });
        }
        Ok(members)
    }

    // Workflow
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

    async fn list_workflows(&self, team_id: Uuid) -> Result<Vec<WorkflowEntity>> {
        let rows = sqlx::query("SELECT id, account_id, name, definition, created_at FROM workflows WHERE account_id = ?")
            .bind(team_id.to_string())
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

    async fn create_credential(&self, name: &str, credential_type: &str, data: &str, team_id: Uuid) -> Result<Credential> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let encrypted_data = super::encryption::encrypt(data)?;

        sqlx::query(
            "INSERT INTO credentials (id, account_id, name, type, data, created_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(team_id.to_string())
        .bind(name)
        .bind(credential_type)
        .bind(&encrypted_data)
        .bind(created_at.to_rfc3339())
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
        let row = sqlx::query("SELECT id, account_id, name, type, data, created_at FROM credentials WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(Credential {
                id: Uuid::parse_str(row.get("id"))?,
                account_id: Uuid::parse_str(row.get("account_id"))?,
                name: row.get("name"),
                credential_type: row.get("type"),
                data: row.get("data"),
                created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_credentials(&self, team_id: Uuid) -> Result<Vec<Credential>> {
        let rows = sqlx::query("SELECT id, account_id, name, type, data, created_at FROM credentials WHERE account_id = ?")
            .bind(team_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        let mut creds = Vec::new();
        for row in rows {
            creds.push(Credential {
                id: Uuid::parse_str(row.get("id"))?,
                account_id: Uuid::parse_str(row.get("account_id"))?,
                name: row.get("name"),
                credential_type: row.get("type"),
                data: row.get("data"),
                created_at: DateTime::parse_from_rfc3339(row.get("created_at"))?.with_timezone(&Utc),
            });
        }
        Ok(creds)
    }

    // Executions
    async fn create_execution(&self, id: Uuid, workflow_id: Option<Uuid>, status: &str) -> Result<Uuid> {
        let started_at = Utc::now();
        let wf_id_str = workflow_id.map(|u| u.to_string());
        
        sqlx::query(
            "INSERT INTO executions (id, workflow_id, status, started_at) VALUES (?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(wf_id_str)
        .bind(status)
        .bind(started_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    async fn update_execution(&self, id: Uuid, status: &str, finished_at: Option<chrono::DateTime<Utc>>, error: Option<String>) -> Result<()> {
        let fin_at_str = finished_at.map(|t| t.to_rfc3339());
        sqlx::query(
            "UPDATE executions SET status = ?, finished_at = ?, error = ? WHERE id = ?"
        )
        .bind(status)
        .bind(fin_at_str)
        .bind(error)
        .bind(id.to_string())
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
        let data = serde_json::to_string(event)?;
        
        sqlx::query(
            "INSERT INTO execution_events (id, execution_id, event_type, data, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(execution_id.to_string())
        .bind(event_type)
        .bind(data)
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_execution(&self, id: Uuid) -> Result<Option<ExecutionRecord>> {
        let row = sqlx::query("SELECT id, workflow_id, status, started_at, finished_at, error FROM executions WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let wf_id_db: Option<String> = row.get("workflow_id");
            let wf_id = if let Some(s) = wf_id_db { Some(Uuid::parse_str(&s)?) } else { None };
            
            let fin_at_db: Option<String> = row.get("finished_at");
            let fin_at = if let Some(s) = fin_at_db { Some(DateTime::parse_from_rfc3339(&s)?.with_timezone(&Utc)) } else { None };

            Ok(Some(ExecutionRecord {
                id: Uuid::parse_str(row.get("id"))?,
                workflow_id: wf_id,
                status: row.get("status"),
                started_at: DateTime::parse_from_rfc3339(row.get("started_at"))?.with_timezone(&Utc),
                finished_at: fin_at,
                error: row.get("error"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_execution_logs(&self, id: Uuid) -> Result<Vec<ExecutionEvent>> {
        let rows = sqlx::query("SELECT data FROM execution_events WHERE execution_id = ? ORDER BY created_at ASC")
            .bind(id.to_string())
            .fetch_all(&self.pool)
            .await?;

        let mut events = Vec::new();
        for row in rows {
            let data_str: String = row.get("data");
            let event: ExecutionEvent = serde_json::from_str(&data_str)?;
            events.push(event);
        }
        Ok(events)
    }
}

