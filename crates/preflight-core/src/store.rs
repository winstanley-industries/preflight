use async_trait::async_trait;

use crate::review::{
    AuthorType, CommentThread, Review, ReviewStatus, ThreadOrigin, ThreadStatus,
};
use uuid::Uuid;

/// Summary of a review for listing.
#[derive(Debug, Clone)]
pub struct ReviewSummary {
    pub id: Uuid,
    pub title: Option<String>,
    pub status: ReviewStatus,
    pub thread_count: usize,
    pub file_count: usize,
}

/// Input for creating a new review.
pub struct CreateReviewInput {
    pub title: Option<String>,
    pub repo_path: String,
    pub base_ref: String,
}

/// Input for creating a new comment thread.
pub struct CreateThreadInput {
    pub review_id: Uuid,
    pub file_path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub origin: ThreadOrigin,
    pub initial_comment_body: String,
    pub initial_comment_author: AuthorType,
    pub revision_number: Option<u32>,
    pub content_snippet: Option<crate::review::ContentSnippet>,
}

/// Input for creating a new revision.
pub struct CreateRevisionInput {
    pub review_id: Uuid,
    pub trigger: crate::review::RevisionTrigger,
    pub message: Option<String>,
    pub files: Vec<crate::diff::FileDiff>,
}

/// Input for adding a comment to a thread.
pub struct AddCommentInput {
    pub thread_id: Uuid,
    pub author_type: AuthorType,
    pub body: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    ReviewNotFound(Uuid),
    ThreadNotFound(Uuid),
    RevisionNotFound(Uuid),
    PersistenceError(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::ReviewNotFound(id) => write!(f, "review not found: {id}"),
            StoreError::ThreadNotFound(id) => write!(f, "thread not found: {id}"),
            StoreError::RevisionNotFound(id) => write!(f, "revision not found: {id}"),
            StoreError::PersistenceError(msg) => write!(f, "persistence error: {msg}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::PersistenceError(e.to_string())
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::PersistenceError(e.to_string())
    }
}

#[async_trait]
pub trait ReviewStore: Send + Sync {
    async fn create_review(&self, input: CreateReviewInput) -> Result<Review, StoreError>;
    async fn get_review(&self, id: Uuid) -> Result<Review, StoreError>;
    async fn list_reviews(&self) -> Vec<ReviewSummary>;
    async fn update_review_status(&self, id: Uuid, status: ReviewStatus) -> Result<(), StoreError>;

    async fn create_thread(&self, input: CreateThreadInput) -> Result<CommentThread, StoreError>;
    async fn get_threads(
        &self,
        review_id: Uuid,
        file_path: Option<&str>,
    ) -> Result<Vec<CommentThread>, StoreError>;
    async fn update_thread_status(
        &self,
        thread_id: Uuid,
        status: ThreadStatus,
    ) -> Result<(), StoreError>;

    async fn add_comment(
        &self,
        input: AddCommentInput,
    ) -> Result<crate::review::Comment, StoreError>;

    async fn create_revision(
        &self,
        input: CreateRevisionInput,
    ) -> Result<crate::review::Revision, StoreError>;
    async fn get_revisions(
        &self,
        review_id: Uuid,
    ) -> Result<Vec<crate::review::Revision>, StoreError>;
    async fn get_revision(
        &self,
        review_id: Uuid,
        revision_number: u32,
    ) -> Result<crate::review::Revision, StoreError>;
    async fn get_latest_revision(
        &self,
        review_id: Uuid,
    ) -> Result<crate::review::Revision, StoreError>;
}
