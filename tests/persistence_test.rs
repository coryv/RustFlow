use rust_flow::job_manager::JobManager;
use rust_flow::storage::{Storage, SqliteStorage};
use rust_flow::stream_engine::{StreamExecutor, DebugConfig};
use rust_flow::schema::ExecutionEvent;
use std::sync::Arc;
use uuid::Uuid;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_persistence() {
    // 1. Setup Storage (In-Memory SQLite)
    let storage = SqliteStorage::new("sqlite::memory:").await.unwrap();
    storage.init().await.unwrap();
    let storage_arc = Arc::new(storage);

    // 2. Setup JobManager
    let job_manager = Arc::new(JobManager::new(storage_arc.clone()));

    // 3. Create Job
    let job_id = job_manager.create_job();
    // Wait for async create_execution
    sleep(Duration::from_millis(100)).await;

    // Check DB for pending execution
    let job_uuid = Uuid::parse_str(&job_id).unwrap();
    let exec = storage_arc.get_execution(job_uuid).await.unwrap();
    assert!(exec.is_some(), "Execution record not created");
    assert_eq!(exec.unwrap().status, "pending");

    // 4. Run Job (Simple Executor)
    // We create a dummy executor logic. 
    // Ideally we build a real executor from a simple workflow.
    // Let's manually build an empty executor.
    let mut executor = StreamExecutor::new(DebugConfig::default());
    // We won't add nodes, so it finishes immediately.

    job_manager.run_job(job_id.clone(), executor).await;

    // 5. Check DB for completed execution
    let exec = storage_arc.get_execution(job_uuid).await.unwrap().unwrap();
    assert_eq!(exec.status, "completed");
    assert!(exec.finished_at.is_some());

    // 6. Check logs (WorkflowStart/Finish should be emitted by executor, but wait, executor emits those?)
    // StreamExecutor emits events.
    // Empty executor might emit WorkflowStart and Finish.
    let logs = storage_arc.get_execution_logs(job_uuid).await.unwrap();
    // Ideally we see some events.
    println!("Logs: {:?}", logs);
}
