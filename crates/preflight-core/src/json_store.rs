use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::review::{Comment, CommentThread, Review, ReviewStatus, ThreadStatus};
use crate::store::{
    AddCommentInput, CreateReviewInput, CreateThreadInput, ReviewStore, ReviewSummary, StoreError,
};

#[derive(Debug, Serialize, Deserialize, Default)]
struct State {
    reviews: HashMap<Uuid, Review>,
    threads: HashMap<Uuid, CommentThread>,
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
            files: input.files,
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
                let thread_count = state
                    .threads
                    .values()
                    .filter(|t| t.review_id == review.id)
                    .count();
                ReviewSummary {
                    id: review.id,
                    title: review.title.clone(),
                    status: review.status.clone(),
                    file_count: review.files.len(),
                    thread_count,
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
        };
        state.threads.insert(thread.id, thread.clone());
        self.persist(&state).await?;
        Ok(thread)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{FileDiff, FileStatus};
    use crate::review::{AuthorType, ThreadOrigin};
    use tempfile::TempDir;

    async fn test_store() -> (JsonFileStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        let store = JsonFileStore::new(&path).await.unwrap();
        (store, dir)
    }

    fn sample_files() -> Vec<FileDiff> {
        vec![FileDiff {
            old_path: Some("src/main.rs".into()),
            new_path: Some("src/main.rs".into()),
            status: FileStatus::Modified,
            hunks: vec![],
        }]
    }

    async fn create_review_with_store(store: &JsonFileStore) -> Review {
        store
            .create_review(CreateReviewInput {
                title: Some("Test".into()),
                files: sample_files(),
                repo_path: None,
                base_ref: None,
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
                files: sample_files(),
                repo_path: None,
                base_ref: None,
            })
            .await
            .unwrap();
        assert_eq!(review.title.as_deref(), Some("Test review"));
        assert_eq!(review.status, ReviewStatus::Open);
        assert_eq!(review.files.len(), 1);
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
                files: sample_files(),
                repo_path: None,
                base_ref: None,
            })
            .await
            .unwrap();
        store
            .create_review(CreateReviewInput {
                title: Some("Second".into()),
                files: vec![],
                repo_path: None,
                base_ref: None,
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
                files: vec![],
                repo_path: None,
                base_ref: None,
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
                    files: sample_files(),
                    repo_path: None,
                    base_ref: None,
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
                files: sample_files(),
                repo_path: Some("/tmp/fake-repo".into()),
                base_ref: Some("HEAD~1".into()),
            })
            .await
            .unwrap();
        assert_eq!(review.repo_path.as_deref(), Some("/tmp/fake-repo"));
        assert_eq!(review.base_ref.as_deref(), Some("HEAD~1"));

        let fetched = store.get_review(review.id).await.unwrap();
        assert_eq!(fetched.repo_path.as_deref(), Some("/tmp/fake-repo"));
        assert_eq!(fetched.base_ref.as_deref(), Some("HEAD~1"));
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
}
