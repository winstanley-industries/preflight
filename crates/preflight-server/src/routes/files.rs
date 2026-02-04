use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use crate::types::{FileDiffResponse, FileListEntry};

pub fn router() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/{id}/files", get(list_files))
        .route("/{id}/files/{*path}", get(get_file_diff))
}

async fn list_files(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<FileListEntry>>, ApiError> {
    let review = state.store.get_review(id).await?;
    let entries: Vec<FileListEntry> = review
        .files
        .iter()
        .map(|f| {
            let path = f
                .new_path
                .clone()
                .unwrap_or_else(|| f.old_path.clone().unwrap_or_default());
            FileListEntry {
                path,
                status: f.status.clone(),
            }
        })
        .collect();
    Ok(Json(entries))
}

async fn get_file_diff(
    State(state): State<AppState>,
    Path((id, file_path)): Path<(Uuid, String)>,
) -> Result<Json<FileDiffResponse>, ApiError> {
    let review = state.store.get_review(id).await?;
    let file_diff = review
        .files
        .iter()
        .find(|f| {
            let effective_path = f
                .new_path
                .as_deref()
                .or(f.old_path.as_deref())
                .unwrap_or_default();
            effective_path == file_path
        })
        .ok_or_else(|| ApiError::NotFound(format!("file not found: {file_path}")))?;

    let path = file_diff
        .new_path
        .clone()
        .unwrap_or_else(|| file_diff.old_path.clone().unwrap_or_default());

    Ok(Json(FileDiffResponse {
        path,
        old_path: file_diff.old_path.clone(),
        status: file_diff.status.clone(),
        hunks: file_diff.hunks.clone(),
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
        // Leak the TempDir so it stays alive for the duration of the test
        Box::leak(Box::new(dir));
        crate::app(std::sync::Arc::new(store))
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Helper: create a review with a real diff and return its ID.
    async fn create_review_with_diff(app: &axum::Router) -> String {
        let diff = "diff --git a/src/main.rs b/src/main.rs\nindex abc..def 100644\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,4 @@\n use std::io;\n+use std::fs;\n \n fn main() {\n";

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/reviews")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "File test review",
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

    #[tokio::test]
    async fn test_list_files_returns_entries() {
        let app = test_app().await;
        let id = create_review_with_diff(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}/files"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let json = body_json(response).await;
        let files = json.as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0]["path"], "src/main.rs");
        assert_eq!(files[0]["status"], "Modified");
    }

    #[tokio::test]
    async fn test_get_file_diff_returns_hunks() {
        let app = test_app().await;
        let id = create_review_with_diff(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}/files/src/main.rs"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let json = body_json(response).await;
        assert_eq!(json["path"], "src/main.rs");
        assert_eq!(json["status"], "Modified");
        assert!(json["hunks"].is_array());
        assert!(!json["hunks"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_file_diff_not_found() {
        let app = test_app().await;
        let id = create_review_with_diff(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}/files/nonexistent.rs"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_files_review_not_found() {
        let app = test_app().await;
        let fake_id = uuid::Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{fake_id}/files"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
