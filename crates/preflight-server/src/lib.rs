use axum::{Router, routing::get};

pub fn app() -> Router {
    Router::new().route("/", get(hello))
}

async fn hello() -> &'static str {
    preflight_core::hello()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_builds() {
        let _app = app();
    }
}
