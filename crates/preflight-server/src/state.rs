use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use preflight_core::highlight::Highlighter;
use preflight_core::review::AgentStatus;
use preflight_core::store::ReviewStore;
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use crate::ws::{WsEvent, WsEventType};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn ReviewStore>,
    pub highlighter: Arc<Highlighter>,
    pub ws_tx: broadcast::Sender<WsEvent>,
    pub agent_status: Arc<Mutex<HashMap<Uuid, AgentStatus>>>,
    pub agent_presence: Arc<AgentPresenceTracker>,
}

struct PresenceState {
    connected: bool,
    disconnect_handle: Option<tokio::task::JoinHandle<()>>,
}

pub struct AgentPresenceTracker {
    inner: Arc<Mutex<HashMap<Uuid, PresenceState>>>,
    ws_tx: broadcast::Sender<WsEvent>,
}

impl AgentPresenceTracker {
    pub fn new(ws_tx: broadcast::Sender<WsEvent>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ws_tx,
        }
    }

    pub async fn register(&self, review_id: Uuid) {
        let mut map = self.inner.lock().await;
        let entry = map.entry(review_id).or_insert(PresenceState {
            connected: false,
            disconnect_handle: None,
        });

        // Cancel any pending disconnect timer
        if let Some(handle) = entry.disconnect_handle.take() {
            handle.abort();
        }

        let was_connected = entry.connected;
        entry.connected = true;

        if !was_connected {
            let _ = self.ws_tx.send(WsEvent {
                event_type: WsEventType::AgentPresenceChanged,
                review_id: review_id.to_string(),
                payload: serde_json::json!({ "connected": true }),
                timestamp: Utc::now(),
            });
        }
    }

    pub async fn deregister(&self, review_id: Uuid) {
        let mut map = self.inner.lock().await;
        if let Some(entry) = map.get_mut(&review_id) {
            // Cancel any existing timer
            if let Some(handle) = entry.disconnect_handle.take() {
                handle.abort();
            }

            let ws_tx = self.ws_tx.clone();
            let inner = self.inner.clone();
            entry.disconnect_handle = Some(tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let mut map = inner.lock().await;
                if let Some(entry) = map.get_mut(&review_id)
                    && entry.connected
                {
                    entry.connected = false;
                    let _ = ws_tx.send(WsEvent {
                        event_type: WsEventType::AgentPresenceChanged,
                        review_id: review_id.to_string(),
                        payload: serde_json::json!({ "connected": false }),
                        timestamp: Utc::now(),
                    });
                }
            }));
        }
    }

    pub async fn is_connected(&self, review_id: Uuid) -> bool {
        let map = self.inner.lock().await;
        map.get(&review_id).map(|s| s.connected).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_broadcasts_connected() {
        let (ws_tx, mut ws_rx) = broadcast::channel(16);
        let tracker = AgentPresenceTracker::new(ws_tx);
        let review_id = Uuid::new_v4();

        tracker.register(review_id).await;

        let event = ws_rx.recv().await.unwrap();
        assert_eq!(event.event_type, WsEventType::AgentPresenceChanged);
        assert_eq!(event.review_id, review_id.to_string());
        assert_eq!(event.payload["connected"], true);
    }

    #[tokio::test]
    async fn test_register_twice_only_broadcasts_once() {
        let (ws_tx, mut ws_rx) = broadcast::channel(16);
        let tracker = AgentPresenceTracker::new(ws_tx);
        let review_id = Uuid::new_v4();

        tracker.register(review_id).await;
        tracker.register(review_id).await;

        // First register should broadcast
        let _event = ws_rx.recv().await.unwrap();
        // Second register should not broadcast (already connected)
        assert!(ws_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_is_connected_default_false() {
        let (ws_tx, _) = broadcast::channel(16);
        let tracker = AgentPresenceTracker::new(ws_tx);

        assert!(!tracker.is_connected(Uuid::new_v4()).await);
    }

    #[tokio::test]
    async fn test_is_connected_after_register() {
        let (ws_tx, _) = broadcast::channel(16);
        let tracker = AgentPresenceTracker::new(ws_tx);
        let review_id = Uuid::new_v4();

        tracker.register(review_id).await;
        assert!(tracker.is_connected(review_id).await);
    }

    #[tokio::test]
    async fn test_deregister_disconnects_after_grace_period() {
        let (ws_tx, mut ws_rx) = broadcast::channel(16);
        let tracker = Arc::new(AgentPresenceTracker::new(ws_tx));
        let review_id = Uuid::new_v4();

        tracker.register(review_id).await;
        let _connect_event = ws_rx.recv().await.unwrap();

        // Still connected immediately after deregister
        tracker.deregister(review_id).await;
        assert!(tracker.is_connected(review_id).await);

        // After grace period, should disconnect
        tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        assert!(!tracker.is_connected(review_id).await);

        let event = ws_rx.recv().await.unwrap();
        assert_eq!(event.payload["connected"], false);
    }

    #[tokio::test]
    async fn test_register_cancels_deregister_grace_period() {
        let (ws_tx, mut ws_rx) = broadcast::channel(16);
        let tracker = Arc::new(AgentPresenceTracker::new(ws_tx));
        let review_id = Uuid::new_v4();

        tracker.register(review_id).await;
        let _connect_event = ws_rx.recv().await.unwrap();

        tracker.deregister(review_id).await;

        // Re-register before grace period expires
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        tracker.register(review_id).await;

        // Wait past the original grace period
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Should still be connected (grace period was cancelled)
        assert!(tracker.is_connected(review_id).await);
    }
}
