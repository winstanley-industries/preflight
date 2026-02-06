use chrono::{DateTime, Utc};
use serde::Serialize;

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
