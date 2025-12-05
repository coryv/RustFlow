use rust_flow::stream_engine::nodes::actions::file_ops::{FileReadNode, FileWriteNode, ListDirNode};
use rust_flow::stream_engine::StreamNode;
use serde_json::json;
use tokio::sync::mpsc;
use tokio::fs;
use std::time::Duration;

#[tokio::test]
async fn test_file_ops_flow() -> anyhow::Result<()> {
    // Setup
    let test_dir = "test_data_file_ops";
    let _ = fs::remove_dir_all(test_dir).await;
    fs::create_dir_all(test_dir).await?;
    
    let file_path = format!("{}/test.txt", test_dir);

    // 1. Write File
    let write_config = json!({
        "path": file_path,
        "content": "Hello {{ name }}",
        "mode": "overwrite"
    });
    let write_node = FileWriteNode::new(write_config);

    let (tx_in, rx_in) = mpsc::channel(1);
    let (tx_out, mut _rx_out) = mpsc::channel(1);

    tokio::spawn(async move {
        tx_in.send(json!({ "name": "World" })).await.unwrap();
    });

    // Run Write
    write_node.run(vec![rx_in], vec![tx_out]).await?; // Should finish after 1 item

    // Verify file content on disk
    let content = fs::read_to_string(&file_path).await?;
    assert_eq!(content, "Hello World");

    // 2. Read File
    let read_config = json!({
        "path": file_path,
        "stream_lines": false
    });
    let read_node = FileReadNode::new(read_config);
    
    let (tx_in_read, rx_in_read) = mpsc::channel(1);
    let (tx_out_read, mut rx_out_read) = mpsc::channel(1);
    
    tokio::spawn(async move {
        tx_in_read.send(json!({})).await.unwrap();
    });

    tokio::spawn(async move {
        read_node.run(vec![rx_in_read], vec![tx_out_read]).await.unwrap();
    });
    
    let result = tokio::time::timeout(Duration::from_secs(2), rx_out_read.recv())
        .await?
        .expect("Should return read result");
    
    assert_eq!(result["content"], "Hello World");
    
    // 3. List Dir
    let list_config = json!({
        "path": test_dir,
        "pattern": "*.txt"
    });
    let list_node = ListDirNode::new(list_config);

    let (tx_in_list, rx_in_list) = mpsc::channel(1);
    let (tx_out_list, mut rx_out_list) = mpsc::channel(1);

    tokio::spawn(async move {
        tx_in_list.send(json!({})).await.unwrap();
    });

    tokio::spawn(async move {
        list_node.run(vec![rx_in_list], vec![tx_out_list]).await.unwrap();
    });

    let result = tokio::time::timeout(Duration::from_secs(2), rx_out_list.recv())
        .await?
        .expect("Should return list result");

    let files = result["files"].as_array().expect("Should be array");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["name"], "test.txt");

    // Cleanup
    let _ = fs::remove_dir_all(test_dir).await;

    Ok(())
}
