use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use crate::types::{CreateReviewRequest, ReviewResponse, UpdateReviewStatusRequest};
use crate::ws::{WsEvent, WsEventType};
use preflight_core::review::{ThreadOrigin, ThreadStatus};
use preflight_core::store::CreateReviewInput;

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{get, patch, post, put};
    axum::Router::new()
        .route(
            "/",
            get(list_reviews)
                .post(create_review)
                .delete(delete_closed_reviews),
        )
        .route("/{id}", get(get_review).delete(delete_review))
        .route("/{id}/status", patch(update_review_status))
        .route("/{id}/agent-status", get(get_agent_presence))
        .route("/{id}/agent-presence", put(update_agent_presence))
        .route("/{id}/request-revision", post(request_revision))
}

async fn create_review(
    State(state): State<AppState>,
    Json(request): Json<CreateReviewRequest>,
) -> Result<Json<ReviewResponse>, ApiError> {
    let repo_path = std::path::Path::new(&request.repo_path);
    let files = preflight_core::git_diff::diff_against_base(repo_path, &request.base_ref)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let review = state
        .store
        .create_review(CreateReviewInput {
            title: request.title,
            repo_path: request.repo_path,
            base_ref: request.base_ref,
        })
        .await?;

    let revision = state
        .store
        .create_revision(preflight_core::store::CreateRevisionInput {
            review_id: review.id,
            trigger: preflight_core::review::RevisionTrigger::Manual,
            message: None,
            files,
        })
        .await?;

    let thread_count = state.store.get_threads(review.id, None).await?.len();
    let response = ReviewResponse {
        id: review.id,
        title: review.title,
        status: review.status,
        file_count: revision.files.len(),
        thread_count,
        open_thread_count: 0,
        revision_count: 1,
        created_at: review.created_at,
        updated_at: review.updated_at,
    };
    let _ = state.ws_tx.send(WsEvent {
        event_type: WsEventType::ReviewCreated,
        review_id: response.id.to_string(),
        payload: serde_json::to_value(&response).unwrap(),
        timestamp: Utc::now(),
    });
    Ok(Json(response))
}

async fn list_reviews(
    State(state): State<AppState>,
) -> Result<Json<Vec<ReviewResponse>>, ApiError> {
    let summaries = state.store.list_reviews().await;
    let mut responses = Vec::with_capacity(summaries.len());
    for summary in summaries {
        let review = state.store.get_review(summary.id).await?;
        let revision_count = state
            .store
            .get_revisions(summary.id)
            .await
            .map(|r| r.len())
            .unwrap_or(0);
        responses.push(ReviewResponse {
            id: review.id,
            title: review.title,
            status: review.status,
            file_count: summary.file_count,
            thread_count: summary.thread_count,
            open_thread_count: summary.open_thread_count,
            revision_count,
            created_at: review.created_at,
            updated_at: review.updated_at,
        });
    }
    Ok(Json(responses))
}

async fn get_review(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ReviewResponse>, ApiError> {
    let review = state.store.get_review(id).await?;
    let threads = state.store.get_threads(id, None).await?;
    let thread_count = threads.len();
    let open_thread_count = threads
        .iter()
        .filter(|t| t.status == ThreadStatus::Open && t.origin != ThreadOrigin::AgentExplanation)
        .count();
    let revisions = state.store.get_revisions(id).await?;
    let file_count = revisions.last().map(|r| r.files.len()).unwrap_or(0);
    Ok(Json(ReviewResponse {
        id: review.id,
        title: review.title,
        status: review.status,
        file_count,
        thread_count,
        open_thread_count,
        revision_count: revisions.len(),
        created_at: review.created_at,
        updated_at: review.updated_at,
    }))
}

async fn update_review_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateReviewStatusRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .store
        .update_review_status(id, request.status.clone())
        .await?;
    let _ = state.ws_tx.send(WsEvent {
        event_type: WsEventType::ReviewStatusChanged,
        review_id: id.to_string(),
        payload: serde_json::json!({ "status": request.status }),
        timestamp: Utc::now(),
    });
    Ok(StatusCode::NO_CONTENT)
}

async fn request_revision(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let review = state.store.get_review(id).await?;
    if review.status != preflight_core::review::ReviewStatus::Open {
        return Err(ApiError::BadRequest("Review is not open".into()));
    }
    let _ = state.ws_tx.send(WsEvent {
        event_type: WsEventType::RevisionRequested,
        review_id: id.to_string(),
        payload: serde_json::json!({}),
        timestamp: Utc::now(),
    });
    Ok(StatusCode::NO_CONTENT)
}

async fn update_agent_presence(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<crate::types::UpdateAgentPresenceRequest>,
) -> Result<StatusCode, ApiError> {
    // Verify review exists
    state.store.get_review(id).await?;
    if request.connected {
        state.agent_presence.register(id).await;
    } else {
        state.agent_presence.deregister(id).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn get_agent_presence(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::types::AgentPresenceResponse>, ApiError> {
    // Verify the review exists
    state.store.get_review(id).await?;
    let connected = state.agent_presence.is_connected(id).await;
    Ok(Json(crate::types::AgentPresenceResponse { connected }))
}

async fn delete_review(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.store.delete_review(id).await?;
    let _ = state.ws_tx.send(WsEvent {
        event_type: WsEventType::ReviewDeleted,
        review_id: id.to_string(),
        payload: serde_json::json!({ "review_id": id }),
        timestamp: Utc::now(),
    });
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_closed_reviews(State(state): State<AppState>) -> Result<StatusCode, ApiError> {
    let deleted_ids = state.store.delete_closed_reviews().await?;
    for id in deleted_ids {
        let _ = state.ws_tx.send(WsEvent {
            event_type: WsEventType::ReviewDeleted,
            review_id: id.to_string(),
            payload: serde_json::json!({ "review_id": id }),
            timestamp: Utc::now(),
        });
    }
    Ok(StatusCode::NO_CONTENT)
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
        // Leak the TempDir so it stays alive for the duration of the test
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

        std::fs::create_dir_all(p.join("src")).unwrap();
        std::fs::write(p.join("src/main.rs"), "fn main() {}\n").unwrap();
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
        std::fs::write(
            p.join("src/main.rs"),
            "use std::io;\n\nfn main() {\n    println!(\"hello\");\n}\n",
        )
        .unwrap();

        let repo_path = p.to_str().unwrap().to_string();
        (dir, repo_path)
    }

    /// Helper: create a review via POST and return its ID.
    async fn create_review_for_test(app: &axum::Router, repo_path: &str) -> String {
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

    #[tokio::test]
    async fn test_create_review_with_git_repo() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();

        let response = app
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
        assert_eq!(json["title"], "Test review");
        assert_eq!(json["status"], "Open");
        assert_eq!(json["file_count"], 1);
        assert_eq!(json["thread_count"], 0);
        assert_eq!(json["open_thread_count"], 0);
        assert_eq!(json["revision_count"], 1);
        assert!(json["id"].is_string());
        assert!(json["created_at"].is_string());
        assert!(json["updated_at"].is_string());
    }

    #[tokio::test]
    async fn test_create_review_bad_repo_path() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/reviews")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "Bad repo",
                            "repo_path": "/tmp/nonexistent_repo_path_xyz",
                            "base_ref": "HEAD"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_reviews() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/reviews")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let json = body_json(response).await;
        assert!(json.is_array());
    }

    #[tokio::test]
    async fn test_get_review_existing() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        let get_response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_response.status(), StatusCode::OK);
        let json = body_json(get_response).await;
        assert_eq!(json["id"], id);
        assert_eq!(json["title"], "Test review");
        assert_eq!(json["revision_count"], 1);
    }

    #[tokio::test]
    async fn test_get_review_not_found() {
        let app = test_app().await;
        let fake_id = uuid::Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{fake_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_review_status() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        let patch_response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/reviews/{id}/status"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "status": "Closed" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(patch_response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_get_review_open_thread_count() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        // Create a thread
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{id}/threads"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "file_path": "src/main.rs",
                            "line_start": 1,
                            "line_end": 1,
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
        let thread_json = body_json(response).await;
        let thread_id = thread_json["id"].as_str().unwrap();

        // GET review — should have 1 open thread
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let json = body_json(response).await;
        assert_eq!(json["thread_count"], 1);
        assert_eq!(json["open_thread_count"], 1);

        // Resolve the thread
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/threads/{thread_id}/status"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "status": "Resolved" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // GET review — should have 0 open threads
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let json = body_json(response).await;
        assert_eq!(json["thread_count"], 1);
        assert_eq!(json["open_thread_count"], 0);
    }

    #[tokio::test]
    async fn test_delete_review() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/reviews/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify it's gone
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_review_not_found() {
        let app = test_app().await;
        let fake_id = uuid::Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/reviews/{fake_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_closed_reviews() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id1 = create_review_for_test(&app, &repo_path).await;
        let id2 = create_review_for_test(&app, &repo_path).await;

        // Close id1
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/reviews/{id1}/status"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "status": "Closed" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Delete closed reviews
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/reviews")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // id1 should be gone, id2 should remain
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id1}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id2}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_request_revision() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{id}/request-revision"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_get_agent_presence_defaults_to_disconnected() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}/agent-status"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["connected"], false);
    }

    #[tokio::test]
    async fn test_get_agent_presence_not_found() {
        let app = test_app().await;
        let fake_id = uuid::Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{fake_id}/agent-status"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_agent_presence() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        // Register agent presence
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/reviews/{id}/agent-presence"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "connected": true }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify agent is connected
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}/agent-status"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["connected"], true);

        // Deregister agent presence
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/reviews/{id}/agent-presence"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "connected": false }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_update_agent_presence_not_found() {
        let app = test_app().await;
        let fake_id = uuid::Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/reviews/{fake_id}/agent-presence"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "connected": true }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_request_revision_closed_review() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        // Close the review first
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/reviews/{id}/status"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "status": "Closed" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{id}/request-revision"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
