use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsEvent {
    pub event_type: WsEventType,
    pub review_id: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WsEventType {
    ReviewCreated,
    ReviewStatusChanged,
    ReviewDeleted,
    RevisionCreated,
    ThreadCreated,
    CommentAdded,
    ThreadStatusChanged,
    ThreadAcknowledged,
    ThreadPoked,
    RevisionRequested,
    AgentPresenceChanged,
}
