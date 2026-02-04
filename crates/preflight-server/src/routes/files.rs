use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use crate::types::{FileContentLine, FileContentResponse, FileDiffResponse, FileListEntry};
use preflight_core::diff::{DiffLine, Hunk, LineKind};

pub fn router() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new()
        .route("/{id}/files", get(list_files))
        .route("/{id}/files/{*path}", get(get_file_diff))
}

pub fn content_router() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new().route("/{id}/content/{*path}", get(get_file_content))
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

    // Reconstruct full file contents and highlight them
    let (old_content, new_content) = reconstruct_file_contents(&file_diff.hunks);
    let old_highlighted = state.highlighter.highlight_file(&old_content, &path);
    let new_highlighted = state.highlighter.highlight_file(&new_content, &path);

    // Map over hunks and populate highlighted field on each line
    let hunks: Vec<Hunk> = file_diff
        .hunks
        .iter()
        .map(|hunk| Hunk {
            old_start: hunk.old_start,
            old_count: hunk.old_count,
            new_start: hunk.new_start,
            new_count: hunk.new_count,
            context: hunk.context.clone(),
            lines: hunk
                .lines
                .iter()
                .map(|line| {
                    let highlighted = match line.kind {
                        LineKind::Removed => line.old_line_no.and_then(|n| {
                            old_highlighted
                                .as_ref()
                                .and_then(|hl| hl.get((n - 1) as usize).cloned())
                        }),
                        LineKind::Added | LineKind::Context | _ => line.new_line_no.and_then(|n| {
                            new_highlighted
                                .as_ref()
                                .and_then(|hl| hl.get((n - 1) as usize).cloned())
                        }),
                    };
                    DiffLine {
                        kind: line.kind.clone(),
                        content: line.content.clone(),
                        old_line_no: line.old_line_no,
                        new_line_no: line.new_line_no,
                        highlighted,
                    }
                })
                .collect(),
        })
        .collect();

    Ok(Json(FileDiffResponse {
        path,
        old_path: file_diff.old_path.clone(),
        status: file_diff.status.clone(),
        hunks,
    }))
}

async fn get_file_content(
    State(state): State<AppState>,
    Path((id, file_path)): Path<(Uuid, String)>,
) -> Result<Json<FileContentResponse>, ApiError> {
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

    // Reconstruct the new file content from all hunks
    let (_, new_content) = reconstruct_file_contents(&file_diff.hunks);
    let highlighted_lines = state.highlighter.highlight_file(&new_content, &path);

    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let language = state.highlighter.language_name(ext).map(|s| s.to_string());

    let lines: Vec<FileContentLine> = new_content
        .lines()
        .enumerate()
        .map(|(i, content)| FileContentLine {
            line_no: (i + 1) as u32,
            content: content.to_string(),
            highlighted: highlighted_lines.as_ref().and_then(|hl| hl.get(i).cloned()),
        })
        .collect();

    Ok(Json(FileContentResponse {
        path,
        language,
        lines,
    }))
}

fn reconstruct_file_contents(hunks: &[Hunk]) -> (String, String) {
    let mut old_lines: BTreeMap<u32, &str> = BTreeMap::new();
    let mut new_lines: BTreeMap<u32, &str> = BTreeMap::new();

    for hunk in hunks {
        for line in &hunk.lines {
            match line.kind {
                LineKind::Context => {
                    if let Some(n) = line.old_line_no {
                        old_lines.insert(n, &line.content);
                    }
                    if let Some(n) = line.new_line_no {
                        new_lines.insert(n, &line.content);
                    }
                }
                LineKind::Removed => {
                    if let Some(n) = line.old_line_no {
                        old_lines.insert(n, &line.content);
                    }
                }
                LineKind::Added | _ => {
                    if let Some(n) = line.new_line_no {
                        new_lines.insert(n, &line.content);
                    }
                }
            }
        }
    }

    let to_content = |lines: &BTreeMap<u32, &str>| -> String {
        if lines.is_empty() {
            return String::new();
        }
        let max_line = *lines.keys().max().unwrap();
        let mut content = String::new();
        for i in 1..=max_line {
            if let Some(line) = lines.get(&i) {
                content.push_str(line);
            }
            content.push('\n');
        }
        content
    };

    (to_content(&old_lines), to_content(&new_lines))
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
