use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::diff::FileDiff;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewStatus {
    Open,
    Closed,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadOrigin {
    Comment,
    ExplanationRequest,
    AgentExplanation,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadStatus {
    Open,
    Resolved,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorType {
    Human,
    Agent,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevisionTrigger {
    Agent,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSnippet {
    pub lines: Vec<String>,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    pub id: Uuid,
    pub review_id: Uuid,
    pub revision_number: u32,
    pub trigger: RevisionTrigger,
    pub message: Option<String>,
    pub files: Vec<FileDiff>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: Uuid,
    pub title: Option<String>,
    pub status: ReviewStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub repo_path: String,
    pub base_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub author_type: AuthorType,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentThread {
    pub id: Uuid,
    pub review_id: Uuid,
    pub file_path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub origin: ThreadOrigin,
    pub status: ThreadStatus,
    pub comments: Vec<Comment>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub revision_number: Option<u32>,
    #[serde(default)]
    pub content_snippet: Option<ContentSnippet>,
}
