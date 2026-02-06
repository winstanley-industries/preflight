mod helpers;

use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;

#[tokio::test]
async fn websocket_client_receives_events() {
    // Bind on an ephemeral port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let app = helpers::test_app().await;

    // Spawn the server
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });

    // Connect WebSocket client
    let (mut ws_stream, _) = connect_async(format!("ws://{addr}/api/ws"))
        .await
        .expect("Failed to connect WebSocket");

    // Trigger a review creation via HTTP
    let repo_path = helpers::setup_test_repo();
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/api/reviews"))
        .json(&serde_json::json!({
            "title": "WS integration test",
            "repo_path": repo_path,
            "base_ref": "HEAD"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Read the WebSocket message
    let msg = tokio::time::timeout(std::time::Duration::from_secs(5), ws_stream.next())
        .await
        .expect("Timed out waiting for WS message")
        .expect("Stream ended")
        .expect("WS error");

    let text = msg.into_text().unwrap();
    let event: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(event["event_type"], "review_created");
    assert!(event["review_id"].is_string());
    assert!(event["timestamp"].is_string());
}
