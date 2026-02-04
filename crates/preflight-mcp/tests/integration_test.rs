use preflight_mcp::client::PreflightClient;

/// These tests require a running preflight server.
/// They're ignored by default and run manually or in CI with a server up.

fn get_test_port() -> u16 {
    std::env::var("PREFLIGHT_TEST_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000)
}

#[tokio::test]
#[ignore = "requires running preflight server"]
async fn test_list_reviews() {
    let client = PreflightClient::new(get_test_port());
    let reviews: serde_json::Value = client.get("/api/reviews").await.unwrap();
    assert!(reviews.is_array());
}

#[tokio::test]
#[ignore = "requires running preflight server"]
async fn test_full_review_flow() {
    let client = PreflightClient::new(get_test_port());

    // Create a review
    let review: serde_json::Value = client
        .post(
            "/api/reviews",
            &serde_json::json!({
                "title": "MCP integration test",
                "diff": "diff --git a/test.rs b/test.rs\nindex abc..def 100644\n--- a/test.rs\n+++ b/test.rs\n@@ -1,3 +1,4 @@\n fn main() {\n+    println!(\"hello\");\n }\n"
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
        .get(&format!(
            "/api/reviews/{review_id}/files/{file_path}"
        ))
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
async fn test_connection_error_message() {
    // Connect to a port where nothing is running
    let client = PreflightClient::new(19999);
    let result: Result<serde_json::Value, _> = client.get("/api/reviews").await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("preflight server not reachable"));
}
