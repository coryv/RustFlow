use rust_flow::stream_engine::nodes::actions::sql_node::SqlNode;
use rust_flow::stream_engine::StreamNode;
use serde_json::json;
use tokio::sync::mpsc;
use sqlx::SqlitePool;
use std::time::Duration;

#[tokio::test]
async fn test_sql_node_sqlite_memory() -> anyhow::Result<()> {
    // strict shared memory url
    let conn_str = "sqlite:file:memdb1?mode=memory&cache=shared";
    
    // 1. Setup DB
    let pool = SqlitePool::connect(conn_str).await?;
    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO users (id, name) VALUES (1, 'Alice')")
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO users (id, name) VALUES (2, 'Bob')")
        .execute(&pool)
        .await?;
    
    // 2. Configure Node
    // implementation: config.get("parameters").and_then(|v| v.as_str())
    let config = json!({
        "connection_string": conn_str,
        "query": "SELECT * FROM users WHERE id = $1",
        "parameters": "[1]" 
    });

    let node = SqlNode::new(config);

    // 3. Mock Input
    let (tx_in, rx_in) = mpsc::channel(1);
    let (tx_out, mut rx_out) = mpsc::channel(1);

    let inputs = vec![rx_in];
    let outputs = vec![tx_out];

    // Send input trigger
    tokio::spawn(async move {
        tx_in.send(json!({})).await.unwrap();
    });

    // 4. Run Node
    tokio::spawn(async move {
        node.run(inputs, outputs).await.unwrap();
    });

    // 5. Verify Output
    let result = tokio::time::timeout(Duration::from_secs(2), rx_out.recv())
        .await?
        .expect("Should receive output");

    println!("Result: {}", serde_json::to_string_pretty(&result)?);

    let rows = result.get("rows").unwrap().as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "Alice");

    Ok(())
}

#[tokio::test]
async fn test_sql_node_params_template() -> anyhow::Result<()> {
    let conn_str = "sqlite:file:memdb2?mode=memory&cache=shared";
    let pool = SqlitePool::connect(conn_str).await?;
    sqlx::query("CREATE TABLE items (id INTEGER, val TEXT)").execute(&pool).await?;
    sqlx::query("INSERT INTO items (id, val) VALUES (10, 'Test')").execute(&pool).await?;

    let config = json!({
        "connection_string": conn_str,
        "query": "SELECT val FROM items WHERE id = $1",
        "parameters": "[ {{ input_id }} ]" // MiniJinja template
    });

    let node = SqlNode::new(config);
    let (tx_in, rx_in) = mpsc::channel(1);
    let (tx_out, mut rx_out) = mpsc::channel(1);

    tokio::spawn(async move {
        tx_in.send(json!({ "input_id": 10 })).await.unwrap();
    });

    tokio::spawn(async move {
        node.run(vec![rx_in], vec![tx_out]).await.unwrap();
    });

    let result = tokio::time::timeout(Duration::from_secs(2), rx_out.recv())
        .await?
        .expect("Should receive output");
    
    let rows = result.get("rows").unwrap().as_array().unwrap();
    assert_eq!(rows[0]["val"], "Test");

    Ok(())
}
