use futures_util::StreamExt;
use preflight_core::ws::WsEvent;
use reqwest::Client;
use serde::de::DeserializeOwned;
use tokio::sync::broadcast;

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

    /// Connect to the API's WebSocket endpoint and spawn a background task
    /// that reads events and rebroadcasts them. Auto-reconnects with
    /// exponential backoff on disconnect.
    pub async fn connect_ws(&self) -> broadcast::Sender<WsEvent> {
        let (tx, _) = broadcast::channel(64);
        let ws_url = format!(
            "ws://{}",
            self.base_url
                .strip_prefix("http://")
                .unwrap_or(&self.base_url)
        );
        let url = format!("{ws_url}/api/ws");
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            let mut backoff = std::time::Duration::from_secs(1);
            let max_backoff = std::time::Duration::from_secs(30);

            loop {
                match tokio_tungstenite::connect_async(&url).await {
                    Ok((ws_stream, _)) => {
                        eprintln!("[mcp] connected to WebSocket at {url}");
                        backoff = std::time::Duration::from_secs(1); // Reset on success
                        let (_write, mut read) = ws_stream.split();

                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                    match serde_json::from_str::<WsEvent>(&text) {
                                        Ok(event) => {
                                            let _ = tx_clone.send(event);
                                        }
                                        Err(e) => {
                                            eprintln!("[mcp] failed to parse WS event: {e}");
                                        }
                                    }
                                }
                                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
                                Err(e) => {
                                    eprintln!("[mcp] WebSocket error: {e}");
                                    break;
                                }
                                _ => {} // Ignore ping/pong/binary
                            }
                        }

                        eprintln!("[mcp] WebSocket disconnected, reconnecting...");
                    }
                    Err(e) => {
                        eprintln!(
                            "[mcp] WebSocket connection failed: {e}, retrying in {:.0}s",
                            backoff.as_secs_f64()
                        );
                    }
                }

                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(max_backoff);
            }
        });

        tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_ws_returns_broadcast_sender() {
        // Start a minimal WS server
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (_write, mut read) = futures_util::StreamExt::split(ws_stream);
            // Just keep the connection alive
            while futures_util::StreamExt::next(&mut read).await.is_some() {}
        });

        let client = PreflightClient::new(port);
        let tx = client.connect_ws().await;
        // Should have a broadcast sender with no receivers yet
        assert_eq!(tx.receiver_count(), 0);
    }

    #[tokio::test]
    async fn connect_ws_receives_events() {
        use preflight_core::ws::{WsEvent, WsEventType};
        use tokio_tungstenite::tungstenite::Message;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();

            // Send a WsEvent as JSON
            let event = WsEvent {
                event_type: WsEventType::CommentAdded,
                review_id: "test-123".to_string(),
                payload: serde_json::json!({"thread_id": "t1"}),
                timestamp: chrono::Utc::now(),
            };
            let json = serde_json::to_string(&event).unwrap();

            use futures_util::SinkExt;
            ws_stream.send(Message::Text(json.into())).await.unwrap();
        });

        let client = PreflightClient::new(port);
        let tx = client.connect_ws().await;
        let mut rx = tx.subscribe();

        let event = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("timed out")
            .expect("recv error");

        assert_eq!(event.review_id, "test-123");
        assert!(matches!(event.event_type, WsEventType::CommentAdded));
    }
}
