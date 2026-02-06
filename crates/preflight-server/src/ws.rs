use axum::{
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::state::AppState;

#[derive(Clone, Debug, Serialize)]
pub struct WsEvent {
    pub event_type: WsEventType,
    pub review_id: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WsEventType {
    ReviewCreated,
    ReviewStatusChanged,
    RevisionCreated,
    ThreadCreated,
    CommentAdded,
    ThreadStatusChanged,
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.ws_tx.subscribe();
    loop {
        match rx.recv().await {
            Ok(event) => {
                if let Ok(json) = serde_json::to_string(&event)
                    && socket.send(Message::Text(json.into())).await.is_err()
                {
                    break; // Client disconnected
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                eprintln!("WebSocket client lagged, skipped {n} messages");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                break; // Channel closed (server shutting down)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_event_serializes_correctly() {
        let event = WsEvent {
            event_type: WsEventType::ReviewCreated,
            review_id: "abc-123".to_string(),
            payload: serde_json::json!({"id": "abc-123"}),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["event_type"], "review_created");
        assert_eq!(parsed["review_id"], "abc-123");
    }

    #[tokio::test]
    async fn broadcast_channel_delivers_events() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<WsEvent>(16);
        let event = WsEvent {
            event_type: WsEventType::ThreadCreated,
            review_id: "test-id".to_string(),
            payload: serde_json::json!({}),
            timestamp: Utc::now(),
        };
        tx.send(event.clone()).unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.review_id, "test-id");
    }
}
