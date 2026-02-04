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
    let files = preflight_core::parser::parse_diff(&request.diff).unwrap_or_default();
    let review = state
        .store
        .create_review(CreateReviewInput {
            title: request.title,
            files,
            repo_path: None,
            base_ref: None,
        })
        .await?;
    Ok(Json(ReviewResponse {
        id: review.id,
        title: review.title,
        status: review.status,
        file_count: review.files.len(),
        thread_count: 0,
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
        responses.push(ReviewResponse {
            id: review.id,
            title: review.title,
            status: review.status,
            file_count: review.files.len(),
            thread_count: summary.thread_count,
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
    Ok(Json(ReviewResponse {
        id: review.id,
        title: review.title,
        status: review.status,
        file_count: review.files.len(),
        thread_count,
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

    #[tokio::test]
    async fn test_create_review_with_valid_diff() {
        let app = test_app().await;

        let diff = "diff --git a/src/main.rs b/src/main.rs\nindex abc..def 100644\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,4 @@\n use std::io;\n+use std::fs;\n \n fn main() {\n";

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/reviews")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "Test review",
                            "diff": diff
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
        assert!(json["id"].is_string());
        assert!(json["created_at"].is_string());
        assert!(json["updated_at"].is_string());
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

        // First, create a review
        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/reviews")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "Get test",
                            "diff": ""
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let created = body_json(create_response).await;
        let id = created["id"].as_str().unwrap();

        // Now fetch it
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
        assert_eq!(json["title"], "Get test");
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

        // Create a review
        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/reviews")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "Status test",
                            "diff": ""
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let created = body_json(create_response).await;
        let id = created["id"].as_str().unwrap();

        // Update its status
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
