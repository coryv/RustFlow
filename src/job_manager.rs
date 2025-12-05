use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use tokio::sync::{mpsc, broadcast};
use crate::stream_engine::StreamExecutor;
use crate::schema::ExecutionEvent;
use crate::storage::Storage;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

#[derive(Debug, Serialize)]
pub struct Job {
    pub id: String,
    pub status: JobStatus,
    pub logs: Vec<String>,
    #[serde(skip)]
    pub log_sender: Option<mpsc::Sender<String>>,
    #[serde(skip)]
    pub event_sender: broadcast::Sender<ExecutionEvent>,
}

pub struct JobManager {
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    storage: Arc<dyn Storage>,
}

// Cannot implement Default because Storage is required
// impl Default for JobManager { ... }

impl JobManager {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            storage,
        }
    }

    pub fn create_job(&self) -> String {
        let id_uuid = Uuid::new_v4();
        let id = id_uuid.to_string();
        
        // Persist initial Pending state
        // We spawn this because create_job is sync currently.
        // Ideally create_job should be async. But to minimize refactor, we spawn.
        let storage = self.storage.clone();
        tokio::spawn(async move {
            let _ = storage.create_execution(id_uuid, None, "pending").await;
        });

        let (tx, mut rx) = mpsc::channel(100);
        let (event_tx, _) = broadcast::channel(100);
        
        let job = Job {
            id: id.clone(),
            status: JobStatus::Pending,
            logs: Vec::new(),
            log_sender: Some(tx),
            event_sender: event_tx,
        };

        let jobs = self.jobs.clone();
        let job_id = id.clone();
        
        // Spawn a task to collect logs
        tokio::spawn(async move {
            while let Some(log) = rx.recv().await {
                let mut jobs_guard = jobs.lock().unwrap();
                if let Some(job) = jobs_guard.get_mut(&job_id) {
                    job.logs.push(log);
                }
            }
        });

        self.jobs.lock().unwrap().insert(id.clone(), job);
        id
    }

    pub fn get_job(&self, id: &str) -> Option<JobStatus> {
        self.jobs.lock().unwrap().get(id).map(|j| j.status.clone())
    }

    pub fn get_job_logs(&self, id: &str) -> Option<Vec<String>> {
        self.jobs.lock().unwrap().get(id).map(|j| j.logs.clone())
    }

    pub fn update_status(&self, id: &str, status: JobStatus) {
        if let Some(job) = self.jobs.lock().unwrap().get_mut(id) {
            job.status = status;
        }
    }

    pub fn subscribe_to_events(&self, id: &str) -> Option<broadcast::Receiver<ExecutionEvent>> {
        self.jobs.lock().unwrap().get(id).map(|j| j.event_sender.subscribe())
    }

    pub async fn run_job(&self, id: String, executor: StreamExecutor) {
        let id_uuid = Uuid::parse_str(&id).unwrap_or_default(); // Should be valid as we generated it

        self.update_status(&id, JobStatus::Running);
        let _ = self.storage.update_execution(id_uuid, "running", None, None).await;

        // Subscribe to events for persistence
        if let Some(mut rx) = self.subscribe_to_events(&id) {
            let storage = self.storage.clone();
            let log_id_uuid = id_uuid;
            tokio::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    let _ = storage.log_execution_event(log_id_uuid, &event).await;
                }
            });
        }
        
        // Inject event sender
        let event_sender = {
            let jobs = self.jobs.lock().unwrap();
            jobs.get(&id).map(|j| j.event_sender.clone())
        };

        if let Some(sender) = event_sender {
            let mut executor = executor;
            executor.set_event_sender(sender);
            
            let result = executor.run().await;
            
            match result {
                Ok(_) => {
                    self.update_status(&id, JobStatus::Completed);
                    let _ = self.storage.update_execution(id_uuid, "completed", Some(Utc::now()), None).await;
                },
                Err(e) => {
                    self.update_status(&id, JobStatus::Failed(e.to_string()));
                    let _ = self.storage.update_execution(id_uuid, "failed", Some(Utc::now()), Some(e.to_string())).await;
                },
            }
        } else {
             self.update_status(&id, JobStatus::Failed("Job not found during run".to_string()));
        }
    }
}
