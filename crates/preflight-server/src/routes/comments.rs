use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use crate::types::{AddCommentRequest, CommentResponse};
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
    Ok(Json(CommentResponse {
        id: comment.id,
        author_type: comment.author_type,
        body: comment.body,
        created_at: comment.created_at,
    }))
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

    /// Helper: create a review and return its id.
    async fn create_review(app: &axum::Router) -> String {
        let diff = "diff --git a/file.txt b/file.txt\nindex abc..def 100644\n--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,4 @@\n line1\n+line2\n line3\n";
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
