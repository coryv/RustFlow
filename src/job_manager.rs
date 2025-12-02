use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use tokio::sync::{mpsc, broadcast};
use crate::stream_engine::StreamExecutor;
use crate::schema::ExecutionEvent;

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
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create_job(&self) -> String {
        let id = Uuid::new_v4().to_string();
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
        self.update_status(&id, JobStatus::Running);
        
        // We need to capture logs from executor. 
        // Currently executor doesn't support log capture injection easily without changing StreamNode trait.
        // For now, we'll just run it.
        // TODO: Inject log sender into executor/nodes.
        
        // Inject event sender
        let event_sender = {
            let jobs = self.jobs.lock().unwrap();
            jobs.get(&id).map(|j| j.event_sender.clone())
        };

        if let Some(sender) = event_sender {
            // We need to pass this to executor.run(). 
            // Since executor.run() signature is fixed in trait/struct, we might need to modify executor.
            // But wait, executor is a struct. We can add a field to it or pass it to run.
            // Let's assume we modify executor.run() or add a setter.
            // Actually, better to pass it to run() or set it before run.
            // Let's assume we will modify executor.run to accept optional event sender or set it.
            // For now, let's modify executor struct to hold it.
            let mut executor = executor;
            executor.set_event_sender(sender);
            
            let result = executor.run().await;
            
            match result {
                Ok(_) => self.update_status(&id, JobStatus::Completed),
                Err(e) => self.update_status(&id, JobStatus::Failed(e.to_string())),
            }
        } else {
             self.update_status(&id, JobStatus::Failed("Job not found during run".to_string()));
        }
    }
}
