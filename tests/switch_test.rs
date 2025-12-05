use rust_flow::stream_engine::nodes::SwitchNode;
use rust_flow::stream_engine::StreamNode;
use serde_json::json;
use tokio::sync::mpsc;
use std::time::Duration;

#[tokio::test]
async fn test_switch_node_flow() -> anyhow::Result<()> {
    // Config: Switch on "value" field.
    // Cases: ["A", "B", "C"]
    // Outputs: 0->A, 1->B, 2->C, 3->Default
    let config = json!({
        "expression": "{{ value }}",
        "cases": ["A", "B", "C"]
    });
    
    let node = SwitchNode::new(config);

    let (tx_in, rx_in) = mpsc::channel(4);
    
    // Outputs
    let (tx_0, mut rx_0) = mpsc::channel(1);
    let (tx_1, mut rx_1) = mpsc::channel(1);
    let (tx_2, mut rx_2) = mpsc::channel(1);
    let (tx_3, mut rx_3) = mpsc::channel(1);

    tokio::spawn(async move {
        tx_in.send(json!({ "value": "A", "id": 1 })).await.unwrap();
        tx_in.send(json!({ "value": "B", "id": 2 })).await.unwrap();
        tx_in.send(json!({ "value": "X", "id": 3 })).await.unwrap(); // Default
    });

    // Run Node
    tokio::spawn(async move {
        // We provide 4 outputs
        node.run(vec![rx_in], vec![tx_0, tx_1, tx_2, tx_3]).await.unwrap();
    });

    // Verify 0 (A)
    let res0 = tokio::time::timeout(Duration::from_secs(1), rx_0.recv()).await?.expect("A");
    assert_eq!(res0["id"], 1);

    // Verify 1 (B)
    let res1 = tokio::time::timeout(Duration::from_secs(1), rx_1.recv()).await?.expect("B");
    assert_eq!(res1["id"], 2);

    // Verify 3 (Default) - Should be X
    let res3 = tokio::time::timeout(Duration::from_secs(1), rx_3.recv()).await?.expect("Default");
    assert_eq!(res3["id"], 3);

    // Verify 2 (C) - Should be empty
    // assert timeout logic? Or just channel empty?
    // We expect no message on channel 2.
    // We can't easily wait for "no message" without delay, but assuming serial processing above:
    assert!(rx_2.try_recv().is_err());

    Ok(())
}
