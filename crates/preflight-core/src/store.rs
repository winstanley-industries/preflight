use crate::diff::FileDiff;
use crate::review::{AuthorType, CommentThread, Review, ReviewStatus, ThreadOrigin, ThreadStatus};
use uuid::Uuid;

/// Summary of a review for listing.
#[derive(Debug, Clone)]
pub struct ReviewSummary {
    pub id: Uuid,
    pub title: Option<String>,
    pub status: ReviewStatus,
    pub file_count: usize,
    pub thread_count: usize,
}

/// Input for creating a new review.
pub struct CreateReviewInput {
    pub title: Option<String>,
    pub files: Vec<FileDiff>,
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
    PersistenceError(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::ReviewNotFound(id) => write!(f, "review not found: {id}"),
            StoreError::ThreadNotFound(id) => write!(f, "thread not found: {id}"),
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

pub trait ReviewStore: Send + Sync {
    fn create_review(&self, input: CreateReviewInput) -> Result<Review, StoreError>;
    fn get_review(&self, id: Uuid) -> Result<Review, StoreError>;
    fn list_reviews(&self) -> Vec<ReviewSummary>;
    fn update_review_status(&self, id: Uuid, status: ReviewStatus) -> Result<(), StoreError>;

    fn create_thread(&self, input: CreateThreadInput) -> Result<CommentThread, StoreError>;
    fn get_threads(
        &self,
        review_id: Uuid,
        file_path: Option<&str>,
    ) -> Result<Vec<CommentThread>, StoreError>;
    fn update_thread_status(&self, thread_id: Uuid, status: ThreadStatus)
    -> Result<(), StoreError>;

    fn add_comment(&self, input: AddCommentInput) -> Result<crate::review::Comment, StoreError>;
}
