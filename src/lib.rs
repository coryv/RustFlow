pub mod stream_engine;
pub mod schema;
pub mod storage;
pub mod node_registry;
pub mod integration_registry;
pub mod integrations;
pub mod job_manager;

// Re-export the main components from stream_engine
pub use stream_engine::{StreamExecutor as Executor, StreamNode as Node};
// We can also re-export nodes for convenience

