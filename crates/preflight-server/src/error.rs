use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// API error type that converts to appropriate HTTP responses.
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = json!({ "error": message });

        (status, axum::Json(body)).into_response()
    }
}

impl From<preflight_core::store::StoreError> for ApiError {
    fn from(err: preflight_core::store::StoreError) -> Self {
        use preflight_core::store::StoreError;
        match err {
            StoreError::ReviewNotFound(id) => ApiError::NotFound(format!("review not found: {id}")),
            StoreError::ThreadNotFound(id) => ApiError::NotFound(format!("thread not found: {id}")),
            StoreError::PersistenceError(msg) => {
                ApiError::Internal(format!("persistence error: {msg}"))
            }
            _ => ApiError::Internal(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use uuid::Uuid;

    #[test]
    fn not_found_produces_404() {
        let err = ApiError::NotFound("missing".into());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn bad_request_produces_400() {
        let err = ApiError::BadRequest("invalid input".into());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn internal_produces_500() {
        let err = ApiError::Internal("something broke".into());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn from_store_review_not_found() {
        let id = Uuid::new_v4();
        let store_err = preflight_core::store::StoreError::ReviewNotFound(id);
        let api_err: ApiError = store_err.into();
        assert!(matches!(api_err, ApiError::NotFound(_)));
    }

    #[test]
    fn from_store_thread_not_found() {
        let id = Uuid::new_v4();
        let store_err = preflight_core::store::StoreError::ThreadNotFound(id);
        let api_err: ApiError = store_err.into();
        assert!(matches!(api_err, ApiError::NotFound(_)));
    }

    #[test]
    fn from_store_persistence_error() {
        let store_err =
            preflight_core::store::StoreError::PersistenceError("disk full".to_string());
        let api_err: ApiError = store_err.into();
        assert!(matches!(api_err, ApiError::Internal(_)));
    }
}
