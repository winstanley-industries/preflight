use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;
use crate::types::{CreateRevisionRequest, RevisionResponse};
use preflight_core::store::CreateRevisionInput;

pub fn router() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new().route("/{id}/revisions", get(list_revisions).post(create_revision))
}

async fn create_revision(
    State(state): State<AppState>,
    Path(review_id): Path<Uuid>,
    Json(request): Json<CreateRevisionRequest>,
) -> Result<Json<RevisionResponse>, ApiError> {
    let review = state.store.get_review(review_id).await?;
    let repo_path = std::path::Path::new(&review.repo_path);
    let files = preflight_core::git_diff::diff_against_base(repo_path, &review.base_ref)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Compare against latest revision's files — reject if no changes
    if let Ok(latest) = state.store.get_latest_revision(review_id).await {
        let old_paths: std::collections::HashSet<_> = latest
            .files
            .iter()
            .map(|f| {
                f.new_path
                    .clone()
                    .unwrap_or_else(|| f.old_path.clone().unwrap_or_default())
            })
            .collect();
        let new_paths: std::collections::HashSet<_> = files
            .iter()
            .map(|f| {
                f.new_path
                    .clone()
                    .unwrap_or_else(|| f.old_path.clone().unwrap_or_default())
            })
            .collect();

        if old_paths == new_paths && latest.files.len() == files.len() {
            // Simple heuristic: check if file count and paths match, then compare hunk contents
            let old_hunks: Vec<_> = latest.files.iter().flat_map(|f| &f.hunks).collect();
            let new_hunks: Vec<_> = files.iter().flat_map(|f| &f.hunks).collect();
            if old_hunks.len() == new_hunks.len() {
                let same = old_hunks.iter().zip(new_hunks.iter()).all(|(a, b)| {
                    a.old_start == b.old_start
                        && a.new_start == b.new_start
                        && a.old_count == b.old_count
                        && a.new_count == b.new_count
                        && a.lines.len() == b.lines.len()
                        && a.lines
                            .iter()
                            .zip(b.lines.iter())
                            .all(|(la, lb)| la.content == lb.content && la.kind == lb.kind)
                });
                if same {
                    return Err(ApiError::BadRequest(
                        "no changes detected since last revision".into(),
                    ));
                }
            }
        }
    }

    let revision = state
        .store
        .create_revision(CreateRevisionInput {
            review_id,
            trigger: request.trigger,
            message: request.message,
            files,
        })
        .await?;

    Ok(Json(RevisionResponse {
        id: revision.id,
        review_id: revision.review_id,
        revision_number: revision.revision_number,
        trigger: revision.trigger,
        message: revision.message,
        file_count: revision.files.len(),
        created_at: revision.created_at,
    }))
}

async fn list_revisions(
    State(state): State<AppState>,
    Path(review_id): Path<Uuid>,
) -> Result<Json<Vec<RevisionResponse>>, ApiError> {
    let revisions = state.store.get_revisions(review_id).await?;
    let responses: Vec<RevisionResponse> = revisions
        .into_iter()
        .map(|r| RevisionResponse {
            id: r.id,
            review_id: r.review_id,
            revision_number: r.revision_number,
            trigger: r.trigger,
            message: r.message,
            file_count: r.files.len(),
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(responses))
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
    async fn test_create_revision() {
        let app = test_app().await;
        let (repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        // Modify the file further so there is a new diff
        std::fs::write(
            repo_dir.path().join("src/main.rs"),
            "use std::io;\nuse std::fs;\n\nfn main() {\n    println!(\"hello\");\n}\n",
        )
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{id}/revisions"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "trigger": "Agent",
                            "message": "Added fs import"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["revision_number"], 2);
        assert_eq!(json["trigger"], "Agent");
        assert_eq!(json["message"], "Added fs import");
        assert!(json["file_count"].as_u64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn test_list_revisions() {
        let app = test_app().await;
        let (repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        // Modify file and create a second revision
        std::fs::write(
            repo_dir.path().join("src/main.rs"),
            "use std::io;\nuse std::fs;\n\nfn main() {\n    println!(\"hello\");\n}\n",
        )
        .unwrap();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{id}/revisions"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "trigger": "Manual"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/reviews/{id}/revisions"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        let revisions = json.as_array().unwrap();
        assert_eq!(revisions.len(), 2);
        assert_eq!(revisions[0]["revision_number"], 1);
        assert_eq!(revisions[1]["revision_number"], 2);
    }

    #[tokio::test]
    async fn test_create_revision_no_changes_returns_400() {
        let app = test_app().await;
        let (_repo_dir, repo_path) = setup_test_repo();
        let id = create_review_for_test(&app, &repo_path).await;

        // Do NOT modify the file — creating a revision should fail
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{id}/revisions"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "trigger": "Manual"
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
    async fn test_create_revision_review_not_found() {
        let app = test_app().await;
        let fake_id = uuid::Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/reviews/{fake_id}/revisions"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "trigger": "Manual"
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
