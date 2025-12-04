use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;
use anyhow::{Result, anyhow};
use crate::storage::{Storage, User, Team, Role, TeamMember, WorkflowEntity, Credential};
use reqwest::Client;
use serde::Serialize;

pub struct RemoteStorage {
    base_url: String,
    client: Client,
    #[allow(dead_code)]
    token: Option<String>, // For future auth
}

impl RemoteStorage {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
            token: None,
        }
    }
    
    // Helper to add auth header if token exists
    // fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder { ... }
}

#[derive(Serialize)]
struct CreateUserRequest {
    username: String,
    password_hash: String,
}

#[derive(Serialize)]
struct CreateTeamRequest {
    name: String,
}

#[async_trait]
impl Storage for RemoteStorage {
    async fn init(&self) -> Result<()> {
        // Check if server is reachable
        let resp = self.client.get(format!("{}/health", self.base_url)).send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Server health check failed: {}", resp.status()))
        }
    }

    // User
    async fn create_user(&self, username: &str, password_hash: &str) -> Result<User> {
        let url = format!("{}/api/auth/register", self.base_url);
        let resp = self.client.post(&url)
            .json(&CreateUserRequest { 
                username: username.to_string(), 
                password_hash: password_hash.to_string() 
            })
            .send()
            .await?;
            
        if resp.status().is_success() {
            Ok(resp.json::<User>().await?)
        } else {
            let err_text = resp.text().await?;
            Err(anyhow!("Failed to register: {}", err_text))
        }
    }

    async fn get_user(&self, _id: Uuid) -> Result<Option<User>> {
        // Not strictly needed for CLI flow yet, but good to have
        Err(anyhow!("Not implemented for remote storage"))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        // Used for login verification in CLI, but for remote, we should probably just hit a login endpoint
        // that returns the user. 
        // However, the CLI `Login` command currently fetches user then verifies hash locally.
        // We need to change that flow or support this endpoint.
        // Let's implement a specific endpoint for fetching user details if needed, 
        // or better, the CLI should just call `login` on the server.
        
        // For now, let's assume we add an endpoint to get user by username (publicly safe? maybe not).
        // Better approach: The `Login` command in CLI needs to be refactored to delegate authentication to the storage.
        // But `Storage` trait has `get_user_by_username`.
        
        // Let's implement a `POST /api/auth/lookup` or similar on server.
        let url = format!("{}/api/auth/lookup?username={}", self.base_url, username);
        let resp = self.client.get(&url).send().await?;
        
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Ok(None)
        } else if resp.status().is_success() {
            Ok(Some(resp.json::<User>().await?))
        } else {
            Err(anyhow!("Failed to lookup user"))
        }
    }

    // Team
    async fn create_team(&self, name: &str) -> Result<Team> {
        let url = format!("{}/api/teams", self.base_url);
        let resp = self.client.post(&url)
            .json(&CreateTeamRequest { name: name.to_string() })
            .send()
            .await?;
            
        if resp.status().is_success() {
            Ok(resp.json::<Team>().await?)
        } else {
            Err(anyhow!("Failed to create team: {}", resp.text().await?))
        }
    }

    async fn get_team(&self, _id: Uuid) -> Result<Option<Team>> {
        Err(anyhow!("Not implemented"))
    }

    async fn get_team_by_name(&self, _name: &str) -> Result<Option<Team>> {
        Err(anyhow!("Not implemented"))
    }

    async fn list_teams_for_user(&self, user_id: Uuid) -> Result<Vec<(Team, Role)>> {
        let url = format!("{}/api/users/{}/teams", self.base_url, user_id);
        let resp = self.client.get(&url).send().await?;
        if resp.status().is_success() {
            Ok(resp.json::<Vec<(Team, Role)>>().await?)
        } else {
            Err(anyhow!("Failed to list teams"))
        }
    }

    // Team Members
    async fn add_team_member(&self, team_id: Uuid, user_id: Uuid, role: Role) -> Result<()> {
        let url = format!("{}/api/teams/{}/members", self.base_url, team_id);
        #[derive(Serialize)]
        struct AddMemberReq { user_id: Uuid, role: String }
        
        let resp = self.client.post(&url)
            .json(&AddMemberReq { user_id, role: role.as_str().to_string() })
            .send()
            .await?;
            
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to add member"))
        }
    }

    async fn get_team_members(&self, team_id: Uuid) -> Result<Vec<TeamMember>> {
        let url = format!("{}/api/teams/{}/members", self.base_url, team_id);
        let resp = self.client.get(&url).send().await?;
        if resp.status().is_success() {
            Ok(resp.json::<Vec<TeamMember>>().await?)
        } else {
            Err(anyhow!("Failed to get members"))
        }
    }

    // Workflow
    async fn save_workflow(&self, workflow: &WorkflowEntity) -> Result<()> {
        let url = format!("{}/api/workflows", self.base_url);
        let resp = self.client.post(&url)
            .json(workflow)
            .send()
            .await?;
            
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to save workflow: {}", resp.text().await?))
        }
    }

    async fn get_workflow(&self, _id: Uuid) -> Result<Option<WorkflowEntity>> {
        Err(anyhow!("Not implemented"))
    }

    async fn list_workflows(&self, team_id: Uuid) -> Result<Vec<WorkflowEntity>> {
        let url = format!("{}/api/teams/{}/workflows", self.base_url, team_id);
        let resp = self.client.get(&url).send().await?;
        if resp.status().is_success() {
            Ok(resp.json::<Vec<WorkflowEntity>>().await?)
        } else {
            Err(anyhow!("Failed to list workflows"))
        }
    }

    // Key-Value
    async fn set_kv(&self, _key: &str, _value: &Value) -> Result<()> {
        Err(anyhow!("Not implemented"))
    }

    async fn get_kv(&self, _key: &str) -> Result<Option<Value>> {
        Err(anyhow!("Not implemented"))
    }

    // Credentials
    async fn create_credential(&self, name: &str, credential_type: &str, data: &str, team_id: Uuid) -> Result<Credential> {
        let url = format!("{}/api/credentials", self.base_url);
        #[derive(Serialize)]
        struct CreateCredReq { name: String, credential_type: String, data: String, team_id: Uuid }
        
        let resp = self.client.post(&url)
            .json(&CreateCredReq { 
                name: name.to_string(), 
                credential_type: credential_type.to_string(), 
                data: data.to_string(), 
                team_id 
            })
            .send()
            .await?;
            
        if resp.status().is_success() {
            Ok(resp.json::<Credential>().await?)
        } else {
            Err(anyhow!("Failed to create credential"))
        }
    }

    async fn get_credential(&self, _id: Uuid) -> Result<Option<Credential>> {
        Err(anyhow!("Not implemented"))
    }

    async fn list_credentials(&self, team_id: Uuid) -> Result<Vec<Credential>> {
        let url = format!("{}/api/teams/{}/credentials", self.base_url, team_id);
        let resp = self.client.get(&url).send().await?;
        if resp.status().is_success() {
            Ok(resp.json::<Vec<Credential>>().await?)
        } else {
            Err(anyhow!("Failed to list credentials"))
        }
    }
}
