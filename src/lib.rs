pub mod stream_engine;
pub mod schema;
pub mod storage;
pub mod integrations;
pub mod job_manager;
pub mod node_registry;

// Re-export the main components from stream_engine
pub use stream_engine::{StreamExecutor as Executor, StreamNode as Node};
// We can also re-export nodes for convenience

