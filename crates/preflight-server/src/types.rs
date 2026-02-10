use chrono::{DateTime, Utc};
use preflight_core::diff::{FileStatus, Hunk};
use preflight_core::review::{AgentStatus, AuthorType, ReviewStatus, ThreadOrigin, ThreadStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- Requests ---

#[derive(Debug, Deserialize)]
pub struct CreateReviewRequest {
    pub title: Option<String>,
    pub repo_path: String,
    pub base_ref: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateRevisionRequest {
    pub trigger: preflight_core::review::RevisionTrigger,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateThreadRequest {
    pub file_path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub origin: ThreadOrigin,
    pub body: String,
    pub author_type: AuthorType,
}

#[derive(Debug, Deserialize)]
pub struct UpdateReviewStatusRequest {
    pub status: ReviewStatus,
}

#[derive(Debug, Deserialize)]
pub struct UpdateThreadStatusRequest {
    pub status: ThreadStatus,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentStatusRequest {
    pub status: AgentStatus,
}

#[derive(Debug, Deserialize)]
pub struct AddCommentRequest {
    pub author_type: AuthorType,
    pub body: String,
}

// --- Query parameters ---

#[derive(Debug, Deserialize)]
pub struct RevisionQuery {
    pub revision: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct InterdiffQuery {
    pub from: u32,
    pub to: u32,
}

// --- Responses ---

#[derive(Debug, Serialize)]
pub struct ReviewResponse {
    pub id: Uuid,
    pub title: Option<String>,
    pub status: ReviewStatus,
    pub file_count: usize,
    pub thread_count: usize,
    pub open_thread_count: usize,
    pub revision_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RevisionResponse {
    pub id: Uuid,
    pub review_id: Uuid,
    pub revision_number: u32,
    pub trigger: preflight_core::review::RevisionTrigger,
    pub message: Option<String>,
    pub file_count: usize,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct FileListEntry {
    pub path: String,
    pub status: FileStatus,
    pub thread_count: usize,
    pub open_thread_count: usize,
}

#[derive(Debug, Serialize)]
pub struct FileDiffResponse {
    pub path: String,
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Serialize)]
pub struct FileContentLine {
    pub line_no: u32,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlighted: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileContentResponse {
    pub path: String,
    pub language: Option<String>,
    pub lines: Vec<FileContentLine>,
}

#[derive(Debug, Serialize)]
pub struct ThreadResponse {
    pub id: Uuid,
    pub review_id: Uuid,
    pub file_path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub origin: ThreadOrigin,
    pub status: ThreadStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_status: Option<AgentStatus>,
    pub comments: Vec<CommentResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub id: Uuid,
    pub author_type: AuthorType,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AgentPresenceResponse {
    pub connected: bool,
}
