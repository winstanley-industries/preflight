use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use crate::types::{CreateReviewRequest, ReviewResponse, UpdateReviewStatusRequest};
use preflight_core::store::CreateReviewInput;

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{get, patch};
    axum::Router::new()
        .route("/", get(list_reviews).post(create_review))
        .route("/{id}", get(get_review))
        .route("/{id}/status", patch(update_review_status))
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
    Ok(Json(ReviewResponse {
        id: review.id,
        title: review.title,
        status: review.status,
        file_count: revision.files.len(),
        thread_count,
        revision_count: 1,
        created_at: review.created_at,
        updated_at: review.updated_at,
    }))
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
    let thread_count = state.store.get_threads(id, None).await?.len();
    let revisions = state.store.get_revisions(id).await?;
    let file_count = revisions.last().map(|r| r.files.len()).unwrap_or(0);
    Ok(Json(ReviewResponse {
        id: review.id,
        title: review.title,
        status: review.status,
        file_count,
        thread_count,
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
    state.store.update_review_status(id, request.status).await?;
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
}
