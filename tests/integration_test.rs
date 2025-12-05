use rust_flow::stream_engine::{StreamExecutor, DebugConfig};
use rust_flow::stream_engine::nodes::{ManualTrigger, SetDataNode, FunctionNode, ConsoleOutputNode};
use serde_json::{json, Value};
use anyhow::Result;

#[tokio::test]
async fn test_basic_workflow() -> Result<()> {
    let mut executor = StreamExecutor::new(DebugConfig::default());

    // Nodes
    executor.add_node("start".to_string(), Box::new(ManualTrigger), None);
    executor.add_node("set_data".to_string(), Box::new(SetDataNode::new(json!({"foo": "bar"}))), None);
    
    executor.add_node("transform".to_string(), Box::new(FunctionNode::new(|mut data: Value| {
        if let Some(obj) = data.as_object_mut() {
            obj.insert("baz".to_string(), json!(42));
        }
        Ok(data)
    })), None);
    
    executor.add_node("print".to_string(), Box::new(ConsoleOutputNode), None);

    // Edges
    executor.add_connection("start".to_string(), 0, "set_data".to_string(), 0);
    executor.add_connection("set_data".to_string(), 0, "transform".to_string(), 0);
    executor.add_connection("transform".to_string(), 0, "print".to_string(), 0);

    // Run
    executor.run().await?;
    
    Ok(())
}
