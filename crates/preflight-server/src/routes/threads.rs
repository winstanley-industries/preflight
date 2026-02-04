use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use crate::types::{
    CommentResponse, CreateThreadRequest, ThreadResponse, UpdateThreadStatusRequest,
};
use preflight_core::store::CreateThreadInput;

/// Routes nested under /api/reviews
pub fn review_router() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new().route("/{id}/threads", get(list_threads).post(create_thread))
}

/// Routes nested under /api/threads
pub fn thread_router() -> axum::Router<AppState> {
    use axum::routing::patch;
    axum::Router::new().route("/{id}/status", patch(update_thread_status))
}

#[derive(Debug, Deserialize)]
struct ThreadFilter {
    file: Option<String>,
}

async fn create_thread(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<CreateThreadRequest>,
) -> Result<Json<ThreadResponse>, ApiError> {
    let input = CreateThreadInput {
        review_id: id,
        file_path: request.file_path,
        line_start: request.line_start,
        line_end: request.line_end,
        origin: request.origin,
        initial_comment_body: request.body,
        initial_comment_author: request.author_type,
    };
    let thread = state.store.create_thread(input).await?;
    Ok(Json(ThreadResponse {
        id: thread.id,
        review_id: thread.review_id,
        file_path: thread.file_path,
        line_start: thread.line_start,
        line_end: thread.line_end,
        origin: thread.origin,
        status: thread.status,
        comments: thread
            .comments
            .into_iter()
            .map(|c| CommentResponse {
                id: c.id,
                author_type: c.author_type,
                body: c.body,
                created_at: c.created_at,
            })
            .collect(),
        created_at: thread.created_at,
        updated_at: thread.updated_at,
    }))
}

async fn list_threads(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(filter): Query<ThreadFilter>,
) -> Result<Json<Vec<ThreadResponse>>, ApiError> {
    let threads = state.store.get_threads(id, filter.file.as_deref()).await?;
    let responses = threads
        .into_iter()
        .map(|thread| ThreadResponse {
            id: thread.id,
            review_id: thread.review_id,
            file_path: thread.file_path,
            line_start: thread.line_start,
            line_end: thread.line_end,
            origin: thread.origin,
            status: thread.status,
            comments: thread
                .comments
                .into_iter()
                .map(|c| CommentResponse {
                    id: c.id,
                    author_type: c.author_type,
                    body: c.body,
                    created_at: c.created_at,
                })
                .collect(),
            created_at: thread.created_at,
            updated_at: thread.updated_at,
        })
        .collect();
    Ok(Json(responses))
}

async fn update_thread_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateThreadStatusRequest>,
) -> Result<StatusCode, ApiError> {
    state.store.update_thread_status(id, request.status).await?;
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
        Box::leak(Box::new(dir));
        crate::app(std::sync::Arc::new(store))
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Helper: create a review and return its id.
    async fn create_review(app: &axum::Router) -> String {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/reviews")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "Thread test review",
                            "diff": ""
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

    /// Helper: create a thread on a review and return the thread response JSON.
    async fn create_thread(app: &axum::Router, review_id: &str) -> serde_json::Value {
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
                            "line_start": 10,
                            "line_end": 15,
                            "origin": "Comment",
                            "body": "Looks good",
                            "author_type": "Human"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        body_json(response).await
    }

    #[tokio::test]
    async fn test_create_thread() {
        let app = test_app().await;
        let review_id = create_review(&app).await;

        let json = create_thread(&app, &review_id).await;

        assert!(json["id"].is_string());
        assert_eq!(json["review_id"], review_id);
        assert_eq!(json["file_path"], "src/main.rs");
        assert_eq!(json["line_start"], 10);
        assert_eq!(json["line_end"], 15);
        assert_eq!(json["origin"], "Comment");
        assert_eq!(json["status"], "Open");
        assert!(json["created_at"].is_string());
        assert!(json["updated_at"].is_string());

        // Should have exactly one initial comment
        let comments = json["comments"].as_array().unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0]["body"], "Looks good");
        assert_eq!(comments[0]["author_type"], "Human");
        assert!(comments[0]["id"].is_string());
        assert!(comments[0]["created_at"].is_string());
    }

    #[tokio::test]
    async fn test_create_thread_unknown_review() {
        let app = test_app().await;
        let fake_id = uuid::Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{fake_id}/threads"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "file_path": "src/main.rs",
                            "line_start": 1,
                            "line_end": 1,
                            "origin": "Comment",
                            "body": "test",
                            "author_type": "Human"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_threads() {
        let app = test_app().await;
        let review_id = create_review(&app).await;

        // Create two threads
        create_thread(&app, &review_id).await;
        create_thread(&app, &review_id).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{review_id}/threads"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[tokio::test]
    async fn test_list_threads_filtered_by_file() {
        let app = test_app().await;
        let review_id = create_review(&app).await;

        // Create a thread on src/main.rs (via helper)
        create_thread(&app, &review_id).await;

        // Create a thread on a different file
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{review_id}/threads"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "file_path": "src/lib.rs",
                            "line_start": 1,
                            "line_end": 5,
                            "origin": "Comment",
                            "body": "Other file",
                            "author_type": "Agent"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Filter by src/main.rs
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{review_id}/threads?file=src/main.rs"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["file_path"], "src/main.rs");
    }

    #[tokio::test]
    async fn test_update_thread_status() {
        let app = test_app().await;
        let review_id = create_review(&app).await;
        let thread_json = create_thread(&app, &review_id).await;
        let thread_id = thread_json["id"].as_str().unwrap();

        let response = app
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

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}
