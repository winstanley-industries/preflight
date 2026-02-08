use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use crate::types::{AddCommentRequest, CommentResponse};
use crate::ws::{WsEvent, WsEventType};
use preflight_core::store::AddCommentInput;

pub fn router() -> axum::Router<AppState> {
    use axum::routing::post;
    axum::Router::new().route("/{id}/comments", post(add_comment))
}

async fn add_comment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<AddCommentRequest>,
) -> Result<Json<CommentResponse>, ApiError> {
    let comment = state
        .store
        .add_comment(AddCommentInput {
            thread_id: id,
            author_type: request.author_type,
            body: request.body,
        })
        .await?;
    // Reset agent status on any new comment:
    // - Human comment means agent needs to re-acknowledge
    // - Agent comment means agent finished working
    state.agent_status.lock().await.remove(&id);
    let response = CommentResponse {
        id: comment.id,
        author_type: comment.author_type,
        body: comment.body,
        created_at: comment.created_at,
    };
    if let Ok(thread) = state.store.get_thread(id).await {
        let _ = state.ws_tx.send(WsEvent {
            event_type: WsEventType::CommentAdded,
            review_id: thread.review_id.to_string(),
            payload: serde_json::json!({
                "thread_id": id.to_string(),
                "comment": serde_json::to_value(&response).unwrap()
            }),
            timestamp: Utc::now(),
        });
    }
    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn test_app() -> axum::Router {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        let store = preflight_core::json_store::JsonFileStore::new(&path)
            .await
            .unwrap();
        Box::leak(Box::new(dir));
        crate::app(std::sync::Arc::new(store))
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Helper: create a temp git repo with a modification, return (TempDir, repo_path_string).
    fn setup_test_repo() -> (tempfile::TempDir, String) {
        use std::process::Command;

        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "t@t.com"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "T"])
            .current_dir(p)
            .output()
            .unwrap();

        std::fs::write(p.join("file.txt"), "line1\nline3\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(p)
            .output()
            .unwrap();

        // Modify the file so there is a diff against HEAD
        std::fs::write(p.join("file.txt"), "line1\nline2\nline3\n").unwrap();

        let repo_path = p.to_str().unwrap().to_string();
        (dir, repo_path)
    }

    /// Helper: create a review and return its id.
    async fn create_review(app: &axum::Router) -> String {
        let (_repo_dir, repo_path) = setup_test_repo();
        // Leak the repo dir so it stays alive for the test
        Box::leak(Box::new(_repo_dir));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/reviews")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "Comment test review",
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

    /// Helper: create a thread on a review and return the thread id.
    async fn create_thread(app: &axum::Router, review_id: &str) -> String {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{review_id}/threads"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "file_path": "file.txt",
                            "line_start": 1,
                            "line_end": 2,
                            "origin": "Comment",
                            "body": "initial comment",
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

    #[tokio::test]
    async fn test_add_comment_success() {
        let app = test_app().await;
        let review_id = create_review(&app).await;
        let thread_id = create_thread(&app, &review_id).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/threads/{thread_id}/comments"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "author_type": "Agent",
                            "body": "Looks good to me!"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let json = body_json(response).await;
        assert_eq!(json["body"], "Looks good to me!");
        assert_eq!(json["author_type"], "Agent");
        assert!(json["id"].is_string());
        assert!(json["created_at"].is_string());
    }

    #[tokio::test]
    async fn test_human_comment_resets_agent_status() {
        let app = test_app().await;
        let review_id = create_review(&app).await;
        let thread_id = create_thread(&app, &review_id).await;

        // Set agent status to Working
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/threads/{thread_id}/agent-status"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "status": "Working" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Add a human comment
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/threads/{thread_id}/comments"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "author_type": "Human",
                            "body": "Another question"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Check agent_status is now null
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{review_id}/threads"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(response).await;
        let threads = json.as_array().unwrap();
        assert!(threads[0]["agent_status"].is_null());
    }

    #[tokio::test]
    async fn test_agent_comment_clears_agent_status() {
        let app = test_app().await;
        let review_id = create_review(&app).await;
        let thread_id = create_thread(&app, &review_id).await;

        // Set agent status to Working
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/threads/{thread_id}/agent-status"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "status": "Working" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Add an agent comment (should reset — agent finished working)
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/threads/{thread_id}/comments"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "author_type": "Agent",
                            "body": "Here's my response"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Check agent_status is cleared
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{review_id}/threads"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(response).await;
        let threads = json.as_array().unwrap();
        assert!(threads[0]["agent_status"].is_null());
    }

    #[tokio::test]
    async fn test_add_comment_thread_not_found() {
        let app = test_app().await;
        let fake_id = uuid::Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/threads/{fake_id}/comments"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "author_type": "Human",
                            "body": "This should fail"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
