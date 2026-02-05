use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use crate::types::{
    FileContentLine, FileContentResponse, FileDiffResponse, FileListEntry, RevisionQuery,
};
use preflight_core::diff::{DiffLine, Hunk, LineKind};
use preflight_core::file_reader;

#[derive(Debug, Deserialize)]
struct ContentQuery {
    version: Option<String>,
}

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
    Query(query): Query<RevisionQuery>,
) -> Result<Json<Vec<FileListEntry>>, ApiError> {
    let revision = match query.revision {
        Some(n) => state.store.get_revision(id, n).await?,
        None => state.store.get_latest_revision(id).await?,
    };
    let entries: Vec<FileListEntry> = revision
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
    Query(query): Query<RevisionQuery>,
) -> Result<Json<FileDiffResponse>, ApiError> {
    let revision = match query.revision {
        Some(n) => state.store.get_revision(id, n).await?,
        None => state.store.get_latest_revision(id).await?,
    };
    let file_diff = revision
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
    Query(query): Query<ContentQuery>,
) -> Result<Json<FileContentResponse>, ApiError> {
    let review = state.store.get_review(id).await?;

    let repo_path = std::path::Path::new(&review.repo_path);
    file_reader::validate_repo_path(repo_path).map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let version = query.version.as_deref().unwrap_or("new");

    // For looking up old_path on renames, use the revision's file list
    let revision = state.store.get_latest_revision(id).await?;

    let (content, path) = match version {
        "old" => {
            let base_ref = &review.base_ref;

            // Check if this is a rename — use the old_path if available
            let read_path = revision
                .files
                .iter()
                .find(|f| {
                    let effective = f
                        .new_path
                        .as_deref()
                        .or(f.old_path.as_deref())
                        .unwrap_or_default();
                    effective == file_path
                })
                .and_then(|f| f.old_path.as_deref())
                .unwrap_or(&file_path);

            let content = file_reader::read_old_file(repo_path, read_path, base_ref)
                .map_err(|e| ApiError::NotFound(e.to_string()))?;
            (content, read_path.to_string())
        }
        _ => {
            let content = file_reader::read_new_file(repo_path, &file_path)
                .map_err(|e| ApiError::NotFound(e.to_string()))?;
            (content, file_path)
        }
    };

    let highlighted_lines = state.highlighter.highlight_file(&content, &path);

    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let language = state.highlighter.language_name(ext).map(|s| s.to_string());

    let lines: Vec<FileContentLine> = content
        .lines()
        .enumerate()
        .map(|(i, line_content)| FileContentLine {
            line_no: (i + 1) as u32,
            content: line_content.to_string(),
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

    /// Helper: create a temp git repo, return its (TempDir, repo_path_string).
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

        // Modify the file
        std::fs::write(
            p.join("src/main.rs"),
            "use std::io;\n\nfn main() {\n    println!(\"hello\");\n}\n",
        )
        .unwrap();

        let repo_path = p.to_str().unwrap().to_string();
        (dir, repo_path)
    }

    /// Helper: create a review using a git repo and return its ID.
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
                            "title": "File test review",
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
    async fn test_list_files_returns_entries() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

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
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

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
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

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

    /// Helper: create a temp git repo with a renamed file, return its (TempDir, repo_path_string).
    fn setup_rename_repo() -> (tempfile::TempDir, String) {
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
        std::fs::write(p.join("src/old_name.rs"), "fn old() {}\n").unwrap();
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

        Command::new("git")
            .args(["mv", "src/old_name.rs", "src/new_name.rs"])
            .current_dir(p)
            .output()
            .unwrap();

        let repo_path = p.to_str().unwrap().to_string();
        (dir, repo_path)
    }

    #[tokio::test]
    async fn test_get_file_content_new_version_from_disk() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}/content/src/main.rs"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        let lines = json["lines"].as_array().unwrap();
        assert_eq!(lines.len(), 5); // 5 lines in the new file
        assert_eq!(lines[0]["content"], "use std::io;");
    }

    #[tokio::test]
    async fn test_get_file_content_old_version_from_git() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}/content/src/main.rs?version=old"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        let lines = json["lines"].as_array().unwrap();
        assert_eq!(lines.len(), 1); // original file was 1 line
        assert_eq!(lines[0]["content"], "fn main() {}");
    }

    #[tokio::test]
    async fn test_get_file_content_old_version_uses_old_path_for_rename() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_rename_repo();

        // Create review pointing to the rename repo — git diff HEAD will show cached rename
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/reviews")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "Rename review",
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
        let id = json["id"].as_str().unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/reviews/{id}/content/src/new_name.rs?version=old"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        let lines = json["lines"].as_array().unwrap();
        assert_eq!(lines[0]["content"], "fn old() {}");
        assert_eq!(json["path"], "src/old_name.rs");
    }

    #[tokio::test]
    async fn test_list_files_with_revision_query() {
        let app = test_app().await;
        let (repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        // Modify file and create revision 2 with an additional file
        std::fs::write(
            repo_dir.path().join("src/main.rs"),
            "use std::io;\nuse std::fs;\n\nfn main() {\n    println!(\"hello\");\n}\n",
        )
        .unwrap();
        std::fs::write(repo_dir.path().join("src/lib.rs"), "pub fn lib() {}\n").unwrap();
        std::process::Command::new("git")
            .args(["add", "src/lib.rs"])
            .current_dir(repo_dir.path())
            .output()
            .unwrap();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{id}/revisions"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "trigger": "Manual" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Query revision 1 — should have 1 file
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}/files?revision=1"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json.as_array().unwrap().len(), 1);

        // Query revision 2 — should have 2 files
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}/files?revision=2"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json.as_array().unwrap().len(), 2);
    }
}
