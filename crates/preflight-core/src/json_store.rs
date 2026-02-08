use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::review::{
    Comment, CommentThread, Review, ReviewStatus, Revision, ThreadOrigin, ThreadStatus,
};
use crate::store::{
    AddCommentInput, CreateReviewInput, CreateRevisionInput, CreateThreadInput, ReviewStore,
    ReviewSummary, StoreError,
};

#[derive(Debug, Serialize, Deserialize, Default)]
struct State {
    reviews: HashMap<Uuid, Review>,
    threads: HashMap<Uuid, CommentThread>,
    #[serde(default)]
    revisions: HashMap<Uuid, Revision>,
}

pub struct JsonFileStore {
    path: PathBuf,
    state: Mutex<State>,
}

impl JsonFileStore {
    pub async fn new(path: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let path = path.into();
        let state = match tokio::fs::read_to_string(&path).await {
            Ok(data) => serde_json::from_str(&data)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => State::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    pub async fn new_empty(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            state: Mutex::new(State::default()),
        }
    }

    async fn persist(&self, state: &State) -> Result<(), StoreError> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let tmp = self.path.with_extension("tmp");
        let data = serde_json::to_string_pretty(state)?;
        tokio::fs::write(&tmp, data).await?;
        tokio::fs::rename(&tmp, &self.path).await?;
        Ok(())
    }
}

#[async_trait]
impl ReviewStore for JsonFileStore {
    async fn create_review(&self, input: CreateReviewInput) -> Result<Review, StoreError> {
        let mut state = self.state.lock().await;
        let now = Utc::now();
        let review = Review {
            id: Uuid::new_v4(),
            title: input.title,
            status: ReviewStatus::Open,
            created_at: now,
            updated_at: now,
            repo_path: input.repo_path,
            base_ref: input.base_ref,
        };
        state.reviews.insert(review.id, review.clone());
        self.persist(&state).await?;
        Ok(review)
    }

    async fn get_review(&self, id: Uuid) -> Result<Review, StoreError> {
        let state = self.state.lock().await;
        state
            .reviews
            .get(&id)
            .cloned()
            .ok_or(StoreError::ReviewNotFound(id))
    }

    // TODO: O(R*T) — pre-build a thread count map if this becomes a hot path
    async fn list_reviews(&self) -> Vec<ReviewSummary> {
        let state = self.state.lock().await;
        state
            .reviews
            .values()
            .map(|review| {
                let review_threads: Vec<_> = state
                    .threads
                    .values()
                    .filter(|t| t.review_id == review.id)
                    .collect();
                let thread_count = review_threads.len();
                let open_thread_count = review_threads
                    .iter()
                    .filter(|t| {
                        t.status == ThreadStatus::Open && t.origin != ThreadOrigin::AgentExplanation
                    })
                    .count();
                let file_count = state
                    .revisions
                    .values()
                    .filter(|r| r.review_id == review.id)
                    .max_by_key(|r| r.revision_number)
                    .map(|r| r.files.len())
                    .unwrap_or(0);
                ReviewSummary {
                    id: review.id,
                    title: review.title.clone(),
                    status: review.status.clone(),
                    thread_count,
                    open_thread_count,
                    file_count,
                }
            })
            .collect()
    }

    async fn update_review_status(&self, id: Uuid, status: ReviewStatus) -> Result<(), StoreError> {
        let mut state = self.state.lock().await;
        let review = state
            .reviews
            .get_mut(&id)
            .ok_or(StoreError::ReviewNotFound(id))?;
        review.status = status;
        review.updated_at = Utc::now();
        self.persist(&state).await?;
        Ok(())
    }

    async fn delete_review(&self, id: Uuid) -> Result<(), StoreError> {
        let mut state = self.state.lock().await;
        if state.reviews.remove(&id).is_none() {
            return Err(StoreError::ReviewNotFound(id));
        }
        state.threads.retain(|_, t| t.review_id != id);
        state.revisions.retain(|_, r| r.review_id != id);
        self.persist(&state).await?;
        Ok(())
    }

    async fn delete_closed_reviews(&self) -> Result<Vec<Uuid>, StoreError> {
        let mut state = self.state.lock().await;
        let closed_ids: Vec<Uuid> = state
            .reviews
            .values()
            .filter(|r| r.status == ReviewStatus::Closed)
            .map(|r| r.id)
            .collect();
        if closed_ids.is_empty() {
            return Ok(vec![]);
        }
        for id in &closed_ids {
            state.reviews.remove(id);
            state.threads.retain(|_, t| t.review_id != *id);
            state.revisions.retain(|_, r| r.review_id != *id);
        }
        self.persist(&state).await?;
        Ok(closed_ids)
    }

    async fn create_thread(&self, input: CreateThreadInput) -> Result<CommentThread, StoreError> {
        let mut state = self.state.lock().await;
        if !state.reviews.contains_key(&input.review_id) {
            return Err(StoreError::ReviewNotFound(input.review_id));
        }
        let now = Utc::now();
        let initial_comment = Comment {
            id: Uuid::new_v4(),
            author_type: input.initial_comment_author,
            body: input.initial_comment_body,
            created_at: now,
        };
        let thread = CommentThread {
            id: Uuid::new_v4(),
            review_id: input.review_id,
            file_path: input.file_path,
            line_start: input.line_start,
            line_end: input.line_end,
            origin: input.origin,
            status: ThreadStatus::Open,
            comments: vec![initial_comment],
            created_at: now,
            updated_at: now,
            revision_number: input.revision_number,
            content_snippet: input.content_snippet,
        };
        state.threads.insert(thread.id, thread.clone());
        self.persist(&state).await?;
        Ok(thread)
    }

    async fn get_thread(&self, thread_id: Uuid) -> Result<CommentThread, StoreError> {
        let state = self.state.lock().await;
        state
            .threads
            .get(&thread_id)
            .cloned()
            .ok_or(StoreError::ThreadNotFound(thread_id))
    }

    async fn get_threads(
        &self,
        review_id: Uuid,
        file_path: Option<&str>,
    ) -> Result<Vec<CommentThread>, StoreError> {
        let state = self.state.lock().await;
        if !state.reviews.contains_key(&review_id) {
            return Err(StoreError::ReviewNotFound(review_id));
        }
        let threads = state
            .threads
            .values()
            .filter(|t| t.review_id == review_id && file_path.is_none_or(|fp| t.file_path == fp))
            .cloned()
            .collect();
        Ok(threads)
    }

    async fn update_thread_status(
        &self,
        thread_id: Uuid,
        status: ThreadStatus,
    ) -> Result<(), StoreError> {
        let mut state = self.state.lock().await;
        let thread = state
            .threads
            .get_mut(&thread_id)
            .ok_or(StoreError::ThreadNotFound(thread_id))?;
        thread.status = status;
        thread.updated_at = Utc::now();
        self.persist(&state).await?;
        Ok(())
    }

    async fn add_comment(&self, input: AddCommentInput) -> Result<Comment, StoreError> {
        let mut state = self.state.lock().await;
        let thread = state
            .threads
            .get_mut(&input.thread_id)
            .ok_or(StoreError::ThreadNotFound(input.thread_id))?;
        let comment = Comment {
            id: Uuid::new_v4(),
            author_type: input.author_type,
            body: input.body,
            created_at: Utc::now(),
        };
        thread.comments.push(comment.clone());
        thread.updated_at = Utc::now();
        self.persist(&state).await?;
        Ok(comment)
    }

    async fn create_revision(&self, input: CreateRevisionInput) -> Result<Revision, StoreError> {
        let mut state = self.state.lock().await;
        if !state.reviews.contains_key(&input.review_id) {
            return Err(StoreError::ReviewNotFound(input.review_id));
        }
        let next_number = state
            .revisions
            .values()
            .filter(|r| r.review_id == input.review_id)
            .map(|r| r.revision_number)
            .max()
            .unwrap_or(0)
            + 1;
        let revision = Revision {
            id: Uuid::new_v4(),
            review_id: input.review_id,
            revision_number: next_number,
            trigger: input.trigger,
            message: input.message,
            files: input.files,
            created_at: Utc::now(),
        };
        state.revisions.insert(revision.id, revision.clone());
        self.persist(&state).await?;
        Ok(revision)
    }

    async fn get_revisions(&self, review_id: Uuid) -> Result<Vec<Revision>, StoreError> {
        let state = self.state.lock().await;
        if !state.reviews.contains_key(&review_id) {
            return Err(StoreError::ReviewNotFound(review_id));
        }
        let mut revisions: Vec<Revision> = state
            .revisions
            .values()
            .filter(|r| r.review_id == review_id)
            .cloned()
            .collect();
        revisions.sort_by_key(|r| r.revision_number);
        Ok(revisions)
    }

    async fn get_revision(
        &self,
        review_id: Uuid,
        revision_number: u32,
    ) -> Result<Revision, StoreError> {
        let state = self.state.lock().await;
        if !state.reviews.contains_key(&review_id) {
            return Err(StoreError::ReviewNotFound(review_id));
        }
        state
            .revisions
            .values()
            .find(|r| r.review_id == review_id && r.revision_number == revision_number)
            .cloned()
            .ok_or(StoreError::RevisionNotFound(review_id))
    }

    async fn get_latest_revision(&self, review_id: Uuid) -> Result<Revision, StoreError> {
        let state = self.state.lock().await;
        if !state.reviews.contains_key(&review_id) {
            return Err(StoreError::ReviewNotFound(review_id));
        }
        state
            .revisions
            .values()
            .filter(|r| r.review_id == review_id)
            .max_by_key(|r| r.revision_number)
            .cloned()
            .ok_or(StoreError::RevisionNotFound(review_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{AuthorType, ThreadOrigin};
    use tempfile::TempDir;

    async fn test_store() -> (JsonFileStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        let store = JsonFileStore::new(&path).await.unwrap();
        (store, dir)
    }

    async fn create_review_with_store(store: &JsonFileStore) -> Review {
        store
            .create_review(CreateReviewInput {
                title: Some("Test".into()),
                repo_path: "/tmp/test-repo".into(),
                base_ref: "HEAD".into(),
            })
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_create_and_get_review() {
        let (store, _dir) = test_store().await;
        let review = store
            .create_review(CreateReviewInput {
                title: Some("Test review".into()),
                repo_path: "/tmp/test-repo".into(),
                base_ref: "HEAD".into(),
            })
            .await
            .unwrap();
        assert_eq!(review.title.as_deref(), Some("Test review"));
        assert_eq!(review.status, ReviewStatus::Open);
        let fetched = store.get_review(review.id).await.unwrap();
        assert_eq!(fetched.id, review.id);
    }

    #[tokio::test]
    async fn test_get_review_not_found() {
        let (store, _dir) = test_store().await;
        let result = store.get_review(Uuid::new_v4()).await;
        assert!(matches!(result, Err(StoreError::ReviewNotFound(_))));
    }

    #[tokio::test]
    async fn test_list_reviews() {
        let (store, _dir) = test_store().await;
        assert!(store.list_reviews().await.is_empty());
        store
            .create_review(CreateReviewInput {
                title: Some("First".into()),
                repo_path: "/tmp/repo1".into(),
                base_ref: "HEAD".into(),
            })
            .await
            .unwrap();
        store
            .create_review(CreateReviewInput {
                title: Some("Second".into()),
                repo_path: "/tmp/repo2".into(),
                base_ref: "HEAD".into(),
            })
            .await
            .unwrap();
        let list = store.list_reviews().await;
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_update_review_status() {
        let (store, _dir) = test_store().await;
        let review = store
            .create_review(CreateReviewInput {
                title: None,
                repo_path: "/tmp/repo".into(),
                base_ref: "HEAD".into(),
            })
            .await
            .unwrap();
        store
            .update_review_status(review.id, ReviewStatus::Closed)
            .await
            .unwrap();
        let updated = store.get_review(review.id).await.unwrap();
        assert_eq!(updated.status, ReviewStatus::Closed);
    }

    #[tokio::test]
    async fn test_persistence_across_instances() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        {
            let store = JsonFileStore::new(&path).await.unwrap();
            store
                .create_review(CreateReviewInput {
                    title: Some("Persisted".into()),
                    repo_path: "/tmp/repo".into(),
                    base_ref: "HEAD".into(),
                })
                .await
                .unwrap();
        }
        {
            let store = JsonFileStore::new(&path).await.unwrap();
            let list = store.list_reviews().await;
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].title.as_deref(), Some("Persisted"));
        }
    }

    #[tokio::test]
    async fn test_create_and_get_thread() {
        let (store, _dir) = test_store().await;
        let review = create_review_with_store(&store).await;
        let thread = store
            .create_thread(CreateThreadInput {
                review_id: review.id,
                file_path: "src/main.rs".into(),
                line_start: 10,
                line_end: 15,
                origin: ThreadOrigin::Comment,
                initial_comment_body: "Looks wrong".into(),
                initial_comment_author: AuthorType::Human,
                revision_number: None,
                content_snippet: None,
            })
            .await
            .unwrap();
        assert_eq!(thread.file_path, "src/main.rs");
        assert_eq!(thread.line_start, 10);
        assert_eq!(thread.line_end, 15);
        assert_eq!(thread.comments.len(), 1);
        assert_eq!(thread.comments[0].body, "Looks wrong");
        assert_eq!(thread.comments[0].author_type, AuthorType::Human);
    }

    #[tokio::test]
    async fn test_create_thread_review_not_found() {
        let (store, _dir) = test_store().await;
        let result = store
            .create_thread(CreateThreadInput {
                review_id: Uuid::new_v4(),
                file_path: "x".into(),
                line_start: 1,
                line_end: 1,
                origin: ThreadOrigin::Comment,
                initial_comment_body: "hi".into(),
                initial_comment_author: AuthorType::Human,
                revision_number: None,
                content_snippet: None,
            })
            .await;
        assert!(matches!(result, Err(StoreError::ReviewNotFound(_))));
    }

    #[tokio::test]
    async fn test_get_threads_review_not_found() {
        let (store, _dir) = test_store().await;
        let result = store.get_threads(Uuid::new_v4(), None).await;
        assert!(matches!(result, Err(StoreError::ReviewNotFound(_))));
    }

    #[tokio::test]
    async fn test_get_threads_filtered_by_file() {
        let (store, _dir) = test_store().await;
        let review = create_review_with_store(&store).await;
        store
            .create_thread(CreateThreadInput {
                review_id: review.id,
                file_path: "src/a.rs".into(),
                line_start: 1,
                line_end: 1,
                origin: ThreadOrigin::Comment,
                initial_comment_body: "a".into(),
                initial_comment_author: AuthorType::Human,
                revision_number: None,
                content_snippet: None,
            })
            .await
            .unwrap();
        store
            .create_thread(CreateThreadInput {
                review_id: review.id,
                file_path: "src/b.rs".into(),
                line_start: 1,
                line_end: 1,
                origin: ThreadOrigin::ExplanationRequest,
                initial_comment_body: "b".into(),
                initial_comment_author: AuthorType::Human,
                revision_number: None,
                content_snippet: None,
            })
            .await
            .unwrap();
        let all = store.get_threads(review.id, None).await.unwrap();
        assert_eq!(all.len(), 2);
        let filtered = store
            .get_threads(review.id, Some("src/a.rs"))
            .await
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].file_path, "src/a.rs");
    }

    #[tokio::test]
    async fn test_update_thread_status() {
        let (store, _dir) = test_store().await;
        let review = create_review_with_store(&store).await;
        let thread = store
            .create_thread(CreateThreadInput {
                review_id: review.id,
                file_path: "src/main.rs".into(),
                line_start: 1,
                line_end: 1,
                origin: ThreadOrigin::Comment,
                initial_comment_body: "fix this".into(),
                initial_comment_author: AuthorType::Human,
                revision_number: None,
                content_snippet: None,
            })
            .await
            .unwrap();
        store
            .update_thread_status(thread.id, ThreadStatus::Resolved)
            .await
            .unwrap();
        let threads = store.get_threads(review.id, None).await.unwrap();
        assert_eq!(threads[0].status, ThreadStatus::Resolved);
    }

    #[tokio::test]
    async fn test_add_comment_to_thread() {
        let (store, _dir) = test_store().await;
        let review = create_review_with_store(&store).await;
        let thread = store
            .create_thread(CreateThreadInput {
                review_id: review.id,
                file_path: "src/main.rs".into(),
                line_start: 1,
                line_end: 1,
                origin: ThreadOrigin::Comment,
                initial_comment_body: "why?".into(),
                initial_comment_author: AuthorType::Human,
                revision_number: None,
                content_snippet: None,
            })
            .await
            .unwrap();
        let comment = store
            .add_comment(AddCommentInput {
                thread_id: thread.id,
                author_type: AuthorType::Agent,
                body: "because X".into(),
            })
            .await
            .unwrap();
        assert_eq!(comment.author_type, AuthorType::Agent);
        assert_eq!(comment.body, "because X");
        let threads = store.get_threads(review.id, None).await.unwrap();
        assert_eq!(threads[0].comments.len(), 2);
    }

    #[tokio::test]
    async fn test_add_comment_thread_not_found() {
        let (store, _dir) = test_store().await;
        let result = store
            .add_comment(AddCommentInput {
                thread_id: Uuid::new_v4(),
                author_type: AuthorType::Human,
                body: "hello".into(),
            })
            .await;
        assert!(matches!(result, Err(StoreError::ThreadNotFound(_))));
    }

    #[tokio::test]
    async fn test_create_review_with_repo_path() {
        let (store, _dir) = test_store().await;
        let review = store
            .create_review(CreateReviewInput {
                title: Some("Repo test".into()),
                repo_path: "/tmp/fake-repo".into(),
                base_ref: "HEAD~1".into(),
            })
            .await
            .unwrap();
        assert_eq!(review.repo_path, "/tmp/fake-repo");
        assert_eq!(review.base_ref, "HEAD~1");

        let fetched = store.get_review(review.id).await.unwrap();
        assert_eq!(fetched.repo_path, "/tmp/fake-repo");
        assert_eq!(fetched.base_ref, "HEAD~1");
    }

    #[tokio::test]
    async fn test_threads_persist() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        let review_id;
        {
            let store = JsonFileStore::new(&path).await.unwrap();
            let review = create_review_with_store(&store).await;
            review_id = review.id;
            store
                .create_thread(CreateThreadInput {
                    review_id: review.id,
                    file_path: "src/main.rs".into(),
                    line_start: 1,
                    line_end: 1,
                    origin: ThreadOrigin::AgentExplanation,
                    initial_comment_body: "This does X".into(),
                    initial_comment_author: AuthorType::Agent,
                    revision_number: None,
                    content_snippet: None,
                })
                .await
                .unwrap();
        }
        {
            let store = JsonFileStore::new(&path).await.unwrap();
            let threads = store.get_threads(review_id, None).await.unwrap();
            assert_eq!(threads.len(), 1);
            assert_eq!(threads[0].origin, ThreadOrigin::AgentExplanation);
        }
    }

    #[tokio::test]
    async fn test_create_and_get_revision() {
        use crate::diff::{FileDiff, FileStatus};
        use crate::review::RevisionTrigger;

        let (store, _dir) = test_store().await;
        let review = create_review_with_store(&store).await;
        let file = FileDiff {
            old_path: None,
            new_path: Some("src/main.rs".into()),
            status: FileStatus::Added,
            hunks: vec![],
        };
        let revision = store
            .create_revision(CreateRevisionInput {
                review_id: review.id,
                trigger: RevisionTrigger::Agent,
                message: Some("Initial diff".into()),
                files: vec![file],
            })
            .await
            .unwrap();
        assert_eq!(revision.revision_number, 1);
        assert_eq!(revision.review_id, review.id);
        assert_eq!(revision.files.len(), 1);
        assert_eq!(revision.message.as_deref(), Some("Initial diff"));

        let fetched = store.get_revision(review.id, 1).await.unwrap();
        assert_eq!(fetched.id, revision.id);
        assert_eq!(fetched.revision_number, 1);
    }

    #[tokio::test]
    async fn test_revision_numbers_increment() {
        use crate::review::RevisionTrigger;

        let (store, _dir) = test_store().await;
        let review = create_review_with_store(&store).await;
        let r1 = store
            .create_revision(CreateRevisionInput {
                review_id: review.id,
                trigger: RevisionTrigger::Agent,
                message: None,
                files: vec![],
            })
            .await
            .unwrap();
        let r2 = store
            .create_revision(CreateRevisionInput {
                review_id: review.id,
                trigger: RevisionTrigger::Manual,
                message: None,
                files: vec![],
            })
            .await
            .unwrap();
        assert_eq!(r1.revision_number, 1);
        assert_eq!(r2.revision_number, 2);
    }

    #[tokio::test]
    async fn test_get_revisions_sorted() {
        use crate::review::RevisionTrigger;

        let (store, _dir) = test_store().await;
        let review = create_review_with_store(&store).await;
        for _ in 0..3 {
            store
                .create_revision(CreateRevisionInput {
                    review_id: review.id,
                    trigger: RevisionTrigger::Agent,
                    message: None,
                    files: vec![],
                })
                .await
                .unwrap();
        }
        let revisions = store.get_revisions(review.id).await.unwrap();
        assert_eq!(revisions.len(), 3);
        assert_eq!(revisions[0].revision_number, 1);
        assert_eq!(revisions[1].revision_number, 2);
        assert_eq!(revisions[2].revision_number, 3);
    }

    #[tokio::test]
    async fn test_get_latest_revision() {
        use crate::review::RevisionTrigger;

        let (store, _dir) = test_store().await;
        let review = create_review_with_store(&store).await;
        store
            .create_revision(CreateRevisionInput {
                review_id: review.id,
                trigger: RevisionTrigger::Agent,
                message: Some("first".into()),
                files: vec![],
            })
            .await
            .unwrap();
        store
            .create_revision(CreateRevisionInput {
                review_id: review.id,
                trigger: RevisionTrigger::Manual,
                message: Some("second".into()),
                files: vec![],
            })
            .await
            .unwrap();
        let latest = store.get_latest_revision(review.id).await.unwrap();
        assert_eq!(latest.revision_number, 2);
        assert_eq!(latest.message.as_deref(), Some("second"));
    }

    #[tokio::test]
    async fn test_create_revision_review_not_found() {
        use crate::review::RevisionTrigger;

        let (store, _dir) = test_store().await;
        let result = store
            .create_revision(CreateRevisionInput {
                review_id: Uuid::new_v4(),
                trigger: RevisionTrigger::Agent,
                message: None,
                files: vec![],
            })
            .await;
        assert!(matches!(result, Err(StoreError::ReviewNotFound(_))));
    }

    #[tokio::test]
    async fn test_corrupted_state_file_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        tokio::fs::write(&path, r#"{"reviews": {"not-a-uuid": "garbage"}}"#)
            .await
            .unwrap();
        let result = JsonFileStore::new(&path).await;
        assert!(matches!(result, Err(StoreError::PersistenceError(_))));
    }

    #[tokio::test]
    async fn test_list_reviews_open_thread_count() {
        let (store, _dir) = test_store().await;
        let review = create_review_with_store(&store).await;
        // Create two threads
        for body in ["first", "second"] {
            store
                .create_thread(CreateThreadInput {
                    review_id: review.id,
                    file_path: "src/main.rs".into(),
                    line_start: 1,
                    line_end: 1,
                    origin: ThreadOrigin::Comment,
                    initial_comment_body: body.into(),
                    initial_comment_author: AuthorType::Human,
                    revision_number: None,
                    content_snippet: None,
                })
                .await
                .unwrap();
        }
        // Both open
        let list = store.list_reviews().await;
        assert_eq!(list[0].thread_count, 2);
        assert_eq!(list[0].open_thread_count, 2);

        // Resolve one
        let threads = store.get_threads(review.id, None).await.unwrap();
        store
            .update_thread_status(threads[0].id, ThreadStatus::Resolved)
            .await
            .unwrap();
        let list = store.list_reviews().await;
        assert_eq!(list[0].thread_count, 2);
        assert_eq!(list[0].open_thread_count, 1);

        // AgentExplanation threads should not count as open
        store
            .create_thread(CreateThreadInput {
                review_id: review.id,
                file_path: "src/main.rs".into(),
                line_start: 1,
                line_end: 1,
                origin: ThreadOrigin::AgentExplanation,
                initial_comment_body: "This does X".into(),
                initial_comment_author: AuthorType::Agent,
                revision_number: None,
                content_snippet: None,
            })
            .await
            .unwrap();
        let list = store.list_reviews().await;
        assert_eq!(list[0].thread_count, 3);
        assert_eq!(list[0].open_thread_count, 1); // still 1, AgentExplanation excluded
    }

    #[tokio::test]
    async fn test_new_empty_ignores_existing_state() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        tokio::fs::write(&path, r#"{"reviews": {"not-a-uuid": "garbage"}}"#)
            .await
            .unwrap();
        let store = JsonFileStore::new_empty(&path).await;
        assert!(store.list_reviews().await.is_empty());
    }

    #[tokio::test]
    async fn test_delete_review_removes_review_and_associated_data() {
        use crate::review::{AuthorType, RevisionTrigger, ThreadOrigin};

        let (store, _dir) = test_store().await;
        let review = create_review_with_store(&store).await;

        store
            .create_thread(CreateThreadInput {
                review_id: review.id,
                file_path: "src/main.rs".into(),
                line_start: 1,
                line_end: 1,
                origin: ThreadOrigin::Comment,
                initial_comment_body: "test".into(),
                initial_comment_author: AuthorType::Human,
                revision_number: None,
                content_snippet: None,
            })
            .await
            .unwrap();
        store
            .create_revision(CreateRevisionInput {
                review_id: review.id,
                trigger: RevisionTrigger::Agent,
                message: None,
                files: vec![],
            })
            .await
            .unwrap();

        store.delete_review(review.id).await.unwrap();

        assert!(matches!(
            store.get_review(review.id).await,
            Err(StoreError::ReviewNotFound(_))
        ));
        assert!(matches!(
            store.get_threads(review.id, None).await,
            Err(StoreError::ReviewNotFound(_))
        ));
        assert!(matches!(
            store.get_revisions(review.id).await,
            Err(StoreError::ReviewNotFound(_))
        ));
        assert!(store.list_reviews().await.is_empty());
    }

    #[tokio::test]
    async fn test_delete_review_not_found() {
        let (store, _dir) = test_store().await;
        let result = store.delete_review(Uuid::new_v4()).await;
        assert!(matches!(result, Err(StoreError::ReviewNotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_review_does_not_affect_other_reviews() {
        let (store, _dir) = test_store().await;
        let review1 = create_review_with_store(&store).await;
        let review2 = create_review_with_store(&store).await;

        store.delete_review(review1.id).await.unwrap();

        assert!(store.get_review(review2.id).await.is_ok());
        assert_eq!(store.list_reviews().await.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_closed_reviews() {
        let (store, _dir) = test_store().await;
        let r1 = create_review_with_store(&store).await;
        let r2 = create_review_with_store(&store).await;
        let r3 = create_review_with_store(&store).await;

        store
            .update_review_status(r1.id, ReviewStatus::Closed)
            .await
            .unwrap();
        store
            .update_review_status(r2.id, ReviewStatus::Closed)
            .await
            .unwrap();

        let deleted = store.delete_closed_reviews().await.unwrap();
        assert_eq!(deleted.len(), 2);
        assert!(deleted.contains(&r1.id));
        assert!(deleted.contains(&r2.id));

        let remaining = store.list_reviews().await;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, r3.id);
    }

    #[tokio::test]
    async fn test_delete_closed_reviews_none_closed() {
        let (store, _dir) = test_store().await;
        create_review_with_store(&store).await;

        let deleted = store.delete_closed_reviews().await.unwrap();
        assert!(deleted.is_empty());
        assert_eq!(store.list_reviews().await.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_review_persists() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        let review_id;
        {
            let store = JsonFileStore::new(&path).await.unwrap();
            let review = create_review_with_store(&store).await;
            review_id = review.id;
            store.delete_review(review.id).await.unwrap();
        }
        {
            let store = JsonFileStore::new(&path).await.unwrap();
            assert!(store.list_reviews().await.is_empty());
            assert!(matches!(
                store.get_review(review_id).await,
                Err(StoreError::ReviewNotFound(_))
            ));
        }
    }
}
