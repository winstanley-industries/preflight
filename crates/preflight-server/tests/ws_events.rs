mod helpers;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower::ServiceExt;

/// Build an app and return both the router and a broadcast receiver for WsEvents.
async fn app_with_ws_rx() -> (
    axum::Router,
    broadcast::Receiver<preflight_server::ws::WsEvent>,
) {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("state.json");
    let store = preflight_core::json_store::JsonFileStore::new(&path)
        .await
        .unwrap();
    Box::leak(Box::new(dir));

    let (ws_tx, ws_rx) = broadcast::channel(64);
    let state = preflight_server::state::AppState {
        store: Arc::new(store),
        highlighter: Arc::new(preflight_core::highlight::Highlighter::new()),
        ws_tx,
        agent_status: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    };

    use axum::routing::get;
    let router = axum::Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .nest("/api/reviews", preflight_server::routes::reviews::router())
        .nest("/api/reviews", preflight_server::routes::files::router())
        .nest(
            "/api/reviews",
            preflight_server::routes::files::content_router(),
        )
        .nest(
            "/api/reviews",
            preflight_server::routes::files::interdiff_router(),
        )
        .nest(
            "/api/reviews",
            preflight_server::routes::revisions::router(),
        )
        .nest(
            "/api/reviews",
            preflight_server::routes::threads::review_router(),
        )
        .nest(
            "/api/threads",
            preflight_server::routes::threads::thread_router(),
        )
        .nest("/api/threads", preflight_server::routes::comments::router())
        .with_state(state);

    (router, ws_rx)
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn create_review_emits_review_created_event() {
    let (app, mut rx) = app_with_ws_rx().await;
    let repo_path = helpers::setup_test_repo();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/reviews")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "WS test",
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
    let review_id = json["id"].as_str().unwrap();

    let event = rx.try_recv().unwrap();
    assert_eq!(event.review_id, review_id);
    assert!(matches!(
        event.event_type,
        preflight_server::ws::WsEventType::ReviewCreated
    ));
}

#[tokio::test]
async fn update_review_status_emits_event() {
    let (app, mut rx) = app_with_ws_rx().await;
    let repo_path = helpers::setup_test_repo();
    let review_id = helpers::create_review(&app, &repo_path).await;
    // Drain the ReviewCreated event
    let _ = rx.try_recv();

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/reviews/{review_id}/status"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({ "status": "Closed" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let event = rx.try_recv().unwrap();
    assert_eq!(event.review_id, review_id);
    assert!(matches!(
        event.event_type,
        preflight_server::ws::WsEventType::ReviewStatusChanged
    ));
    assert_eq!(event.payload["status"], "Closed");
}

#[tokio::test]
async fn create_thread_emits_event() {
    let (app, mut rx) = app_with_ws_rx().await;
    let repo_path = helpers::setup_test_repo();
    let review_id = helpers::create_review(&app, &repo_path).await;
    let _ = rx.try_recv(); // drain ReviewCreated

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/reviews/{review_id}/threads"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "file_path": "src/main.rs",
                        "line_start": 1,
                        "line_end": 2,
                        "origin": "Comment",
                        "body": "ws test thread",
                        "author_type": "Human"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let event = rx.try_recv().unwrap();
    assert_eq!(event.review_id, review_id);
    assert!(matches!(
        event.event_type,
        preflight_server::ws::WsEventType::ThreadCreated
    ));
}

#[tokio::test]
async fn add_comment_emits_event() {
    let (app, mut rx) = app_with_ws_rx().await;
    let repo_path = helpers::setup_test_repo();
    let review_id = helpers::create_review(&app, &repo_path).await;
    let _ = rx.try_recv(); // drain ReviewCreated
    let thread_id = helpers::create_thread(&app, &review_id).await;
    let _ = rx.try_recv(); // drain ThreadCreated

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/threads/{thread_id}/comments"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "author_type": "Agent",
                        "body": "WS comment test"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let event = rx.try_recv().unwrap();
    assert_eq!(event.review_id, review_id);
    assert!(matches!(
        event.event_type,
        preflight_server::ws::WsEventType::CommentAdded
    ));
    assert_eq!(event.payload["thread_id"], thread_id);
}

#[tokio::test]
async fn update_thread_status_emits_event() {
    let (app, mut rx) = app_with_ws_rx().await;
    let repo_path = helpers::setup_test_repo();
    let review_id = helpers::create_review(&app, &repo_path).await;
    let _ = rx.try_recv(); // drain ReviewCreated
    let thread_id = helpers::create_thread(&app, &review_id).await;
    let _ = rx.try_recv(); // drain ThreadCreated

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

    let event = rx.try_recv().unwrap();
    assert_eq!(event.review_id, review_id);
    assert!(matches!(
        event.event_type,
        preflight_server::ws::WsEventType::ThreadStatusChanged
    ));
    assert_eq!(event.payload["thread_id"], thread_id);
    assert_eq!(event.payload["status"], "Resolved");
}
