use std::sync::Arc;

use axum::{
    Router,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use preflight_core::store::ReviewStore;
use rust_embed::RustEmbed;

pub mod error;
pub mod routes;
pub mod state;
pub mod types;

#[derive(RustEmbed)]
#[folder = "../../frontend/dist"]
struct Assets;

pub fn app(store: Arc<dyn ReviewStore>) -> Router {
    let state = state::AppState {
        store,
        highlighter: Arc::new(preflight_core::highlight::Highlighter::new()),
    };
    Router::new()
        .route("/api/health", get(health))
        .nest("/api/reviews", routes::reviews::router())
        .nest("/api/reviews", routes::files::router())
        .nest("/api/reviews", routes::files::content_router())
        .nest("/api/reviews", routes::threads::review_router())
        .nest("/api/threads", routes::threads::thread_router())
        .nest("/api/threads", routes::comments::router())
        .fallback(static_handler)
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn static_handler(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try the exact path first
    if !path.is_empty()
        && let Some(file) = Assets::get(path)
    {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, mime.as_ref())],
            file.data,
        )
            .into_response();
    }

    // SPA fallback: serve index.html for any unmatched route
    match Assets::get("index.html") {
        Some(file) => Html(file.data).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            "index.html not found in embedded assets",
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_builds() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        let store = preflight_core::json_store::JsonFileStore::new(&path)
            .await
            .unwrap();
        let _app = app(std::sync::Arc::new(store));
    }
}
