use reqwest::Client;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone)]
pub struct PreflightClient {
    http: Client,
    base_url: String,
}

#[derive(Debug)]
pub enum ClientError {
    /// The preflight server is not running or unreachable.
    ConnectionFailed(String),
    /// The server returned a non-success status.
    ApiError { status: u16, body: String },
    /// Failed to deserialize the response.
    DeserializeError(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::ConnectionFailed(msg) => write!(
                f,
                "preflight server not reachable at {msg} — start it with `preflight serve`"
            ),
            ClientError::ApiError { status, body } => {
                write!(f, "API error (HTTP {status}): {body}")
            }
            ClientError::DeserializeError(msg) => write!(f, "failed to parse response: {msg}"),
        }
    }
}

impl PreflightClient {
    pub fn new(port: u16) -> Self {
        Self {
            http: Client::new(),
            base_url: format!("http://127.0.0.1:{port}"),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let url = format!("{}{path}", self.base_url);
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::ConnectionFailed(format!("{}: {e}", self.base_url)))?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ClientError::ApiError { status, body });
        }

        response
            .json()
            .await
            .map_err(|e| ClientError::DeserializeError(e.to_string()))
    }

    pub async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, ClientError> {
        let url = format!("{}{path}", self.base_url);
        let response = self
            .http
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| ClientError::ConnectionFailed(format!("{}: {e}", self.base_url)))?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ClientError::ApiError { status, body });
        }

        response
            .json()
            .await
            .map_err(|e| ClientError::DeserializeError(e.to_string()))
    }
}
