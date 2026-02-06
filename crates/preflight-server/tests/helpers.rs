#![allow(dead_code)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::ServiceExt;

/// Build a fresh app with an ephemeral JSON store.
pub async fn test_app() -> axum::Router {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("state.json");
    let store = preflight_core::json_store::JsonFileStore::new(&path)
        .await
        .unwrap();
    Box::leak(Box::new(dir));
    preflight_server::app(Arc::new(store))
}

/// Deserialize an axum response body as JSON.
pub async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Create a temp git repo with one committed file and a working-tree modification.
/// Returns repo_path_string. The TempDir is leaked to keep it alive.
pub fn setup_test_repo() -> String {
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

/// POST /api/reviews with a test repo and return the review ID.
pub async fn create_review(app: &axum::Router, repo_path: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/reviews")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "Test review",
                        "repo_path": repo_path,
                        "base_ref": "HEAD"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    json["id"].as_str().unwrap().to_string()
}

/// POST a thread on a review and return the thread ID.
pub async fn create_thread(app: &axum::Router, review_id: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/reviews/{review_id}/threads"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "file_path": "src/main.rs",
                        "line_start": 1,
                        "line_end": 2,
                        "origin": "Comment",
                        "body": "test comment",
                        "author_type": "Human"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    json["id"].as_str().unwrap().to_string()
}
