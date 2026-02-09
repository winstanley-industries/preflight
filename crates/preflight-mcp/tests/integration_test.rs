use preflight_mcp::client::PreflightClient;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Spin up an ephemeral preflight server and return its port.
async fn start_server() -> u16 {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("state.json");
    let store = preflight_core::json_store::JsonFileStore::new(&path)
        .await
        .unwrap();
    Box::leak(Box::new(dir));

    let app = preflight_server::app(Arc::new(store));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });

    port
}

/// Create a temp git repo with one committed file and a working-tree change.
fn setup_test_repo() -> String {
    use std::process::Command;

    let dir = tempfile::TempDir::new().unwrap();
    let p = dir.path().to_owned();

    Command::new("git")
        .args(["init"])
        .current_dir(&p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "t@t.com"])
        .current_dir(&p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "T"])
        .current_dir(&p)
        .output()
        .unwrap();

    std::fs::create_dir_all(p.join("src")).unwrap();
    std::fs::write(p.join("src/main.rs"), "fn main() {}\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(&p)
        .output()
        .unwrap();

    std::fs::write(
        p.join("src/main.rs"),
        "use std::io;\n\nfn main() {\n    println!(\"hello\");\n}\n",
    )
    .unwrap();

    let repo_path = p.to_str().unwrap().to_string();
    Box::leak(Box::new(dir));
    repo_path
}

#[tokio::test]
async fn test_list_reviews() {
    let port = start_server().await;
    let client = PreflightClient::new(port);
    let reviews: serde_json::Value = client.get("/api/reviews").await.unwrap();
    assert!(reviews.is_array());
}

#[tokio::test]
async fn test_full_review_flow() {
    let port = start_server().await;
    let client = PreflightClient::new(port);
    let repo_path = setup_test_repo();

    // Create a review
    let review: serde_json::Value = client
        .post(
            "/api/reviews",
            &serde_json::json!({
                "title": "MCP integration test",
                "repo_path": repo_path,
                "base_ref": "HEAD"
            }),
        )
        .await
        .unwrap();

    let review_id = review["id"].as_str().unwrap();

    // Get the review
    let fetched: serde_json::Value = client
        .get(&format!("/api/reviews/{review_id}"))
        .await
        .unwrap();
    assert_eq!(fetched["id"].as_str().unwrap(), review_id);

    // List files
    let files: serde_json::Value = client
        .get(&format!("/api/reviews/{review_id}/files"))
        .await
        .unwrap();
    assert!(!files.as_array().unwrap().is_empty());

    // Get diff for a file
    let file_path = files.as_array().unwrap()[0]["path"].as_str().unwrap();
    let diff: serde_json::Value = client
        .get(&format!("/api/reviews/{review_id}/files/{file_path}"))
        .await
        .unwrap();
    assert_eq!(diff["path"].as_str().unwrap(), file_path);

    // Create a comment thread
    let thread: serde_json::Value = client
        .post(
            &format!("/api/reviews/{review_id}/threads"),
            &serde_json::json!({
                "file_path": file_path,
                "line_start": 2,
                "line_end": 2,
                "origin": "Comment",
                "body": "Why this change?",
                "author_type": "Human"
            }),
        )
        .await
        .unwrap();

    let thread_id = thread["id"].as_str().unwrap();

    // Get comments
    let threads: serde_json::Value = client
        .get(&format!("/api/reviews/{review_id}/threads"))
        .await
        .unwrap();
    assert!(!threads.as_array().unwrap().is_empty());

    // Reply as agent
    let comment: serde_json::Value = client
        .post(
            &format!("/api/threads/{thread_id}/comments"),
            &serde_json::json!({
                "author_type": "Agent",
                "body": "This adds a greeting to the output."
            }),
        )
        .await
        .unwrap();
    assert_eq!(comment["author_type"].as_str().unwrap(), "Agent");
}

#[tokio::test]
async fn test_patch_method() {
    let port = start_server().await;
    let client = PreflightClient::new(port);
    let repo_path = setup_test_repo();

    // Create a review first
    let review: serde_json::Value = client
        .post(
            "/api/reviews",
            &serde_json::json!({
                "title": "Patch test",
                "repo_path": repo_path,
                "base_ref": "HEAD"
            }),
        )
        .await
        .unwrap();
    let review_id = review["id"].as_str().unwrap();

    // Patch its status
    client
        .patch(
            &format!("/api/reviews/{review_id}/status"),
            &serde_json::json!({ "status": "Closed" }),
        )
        .await
        .unwrap();

    // Verify it's closed
    let fetched: serde_json::Value = client
        .get(&format!("/api/reviews/{review_id}"))
        .await
        .unwrap();
    assert_eq!(fetched["status"].as_str().unwrap(), "Closed");
}

#[tokio::test]
async fn test_connection_error_message() {
    // Connect to a port where nothing is running
    let client = PreflightClient::new(19999);
    let result: Result<serde_json::Value, _> = client.get("/api/reviews").await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("preflight server not reachable"));
}
