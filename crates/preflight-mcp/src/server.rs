use preflight_core::ws::{WsEvent, WsEventType};
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;
use tokio::sync::broadcast;

use crate::client::{ClientError, PreflightClient};

#[derive(Debug, Clone)]
pub struct PreflightMcp {
    client: PreflightClient,
    tool_router: ToolRouter<Self>,
    pub ws_tx: broadcast::Sender<WsEvent>,
}

// --- Tool input schemas ---

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListReviewsInput {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetReviewInput {
    #[schemars(description = "UUID of the review")]
    pub review_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetDiffInput {
    #[schemars(description = "UUID of the review")]
    pub review_id: String,
    #[schemars(description = "Path of the file within the review (e.g. src/main.rs)")]
    pub file_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCommentsInput {
    #[schemars(description = "UUID of the review")]
    pub review_id: String,
    #[schemars(description = "Optional file path to filter comments by")]
    pub file_path: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RespondToCommentInput {
    #[schemars(description = "UUID of the comment thread to reply to")]
    pub thread_id: String,
    #[schemars(description = "The response text")]
    pub body: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SubmitRevisionInput {
    #[schemars(description = "UUID of the review to create a new revision for")]
    pub review_id: String,
    #[schemars(description = "Description of what was changed")]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateReviewInput {
    #[schemars(description = "Absolute path to the git repository")]
    pub repo_path: String,
    #[schemars(description = "Optional title for the review")]
    pub title: Option<String>,
    #[schemars(description = "Git ref to diff against (e.g. HEAD, main). Defaults to HEAD.")]
    pub base_ref: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateThreadInput {
    #[schemars(description = "UUID of the review")]
    pub review_id: String,
    #[schemars(description = "Path of the file to comment on (e.g. src/main.rs)")]
    pub file_path: String,
    #[schemars(description = "Starting line number")]
    pub line_start: u32,
    #[schemars(description = "Ending line number")]
    pub line_end: u32,
    #[schemars(description = "The comment text")]
    pub body: String,
    #[schemars(
        description = "Thread origin: 'Comment' for regular comments, 'AgentExplanation' for proactive code explanations. Defaults to 'Comment'."
    )]
    pub origin: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateReviewStatusInput {
    #[schemars(description = "UUID of the review")]
    pub review_id: String,
    #[schemars(description = "New status: 'Open' or 'Closed'")]
    pub status: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ResolveThreadInput {
    #[schemars(description = "UUID of the comment thread")]
    pub thread_id: String,
    #[schemars(description = "New status: 'Open' or 'Resolved'")]
    pub status: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WaitForEventInput {
    #[schemars(
        description = "Optional review UUID to filter events. If omitted, matches events from any review."
    )]
    pub review_id: Option<String>,
    #[schemars(
        description = "Optional list of event types to filter. Valid values: review_created, review_status_changed, revision_created, thread_created, comment_added, thread_status_changed. If omitted, matches any event type."
    )]
    pub event_types: Option<Vec<String>>,
    #[schemars(description = "Timeout in seconds. Defaults to 300 (5 minutes). Max 600.")]
    pub timeout_secs: Option<u64>,
}

fn format_error(e: ClientError) -> String {
    e.to_string()
}

fn event_type_matches(event_type: &WsEventType, filter: &str) -> bool {
    match filter {
        "review_created" => matches!(event_type, WsEventType::ReviewCreated),
        "review_status_changed" => matches!(event_type, WsEventType::ReviewStatusChanged),
        "revision_created" => matches!(event_type, WsEventType::RevisionCreated),
        "thread_created" => matches!(event_type, WsEventType::ThreadCreated),
        "comment_added" => matches!(event_type, WsEventType::CommentAdded),
        "thread_status_changed" => matches!(event_type, WsEventType::ThreadStatusChanged),
        _ => false,
    }
}

impl PreflightMcp {
    pub fn new(client: PreflightClient, ws_tx: broadcast::Sender<WsEvent>) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
            ws_tx,
        }
    }
}

#[tool_router]
impl PreflightMcp {
    #[tool(description = "List all active code reviews")]
    async fn list_reviews(
        &self,
        #[allow(unused_variables)] Parameters(_input): Parameters<ListReviewsInput>,
    ) -> Result<String, String> {
        let reviews: serde_json::Value = self
            .client
            .get("/api/reviews")
            .await
            .map_err(format_error)?;

        serde_json::to_string_pretty(&reviews).map_err(|e| e.to_string())
    }

    #[tool(description = "Get a review's metadata and list of changed files")]
    async fn get_review(
        &self,
        Parameters(input): Parameters<GetReviewInput>,
    ) -> Result<String, String> {
        let review: serde_json::Value = self
            .client
            .get(&format!("/api/reviews/{}", input.review_id))
            .await
            .map_err(format_error)?;

        let files: serde_json::Value = self
            .client
            .get(&format!("/api/reviews/{}/files", input.review_id))
            .await
            .map_err(format_error)?;

        let combined = serde_json::json!({
            "review": review,
            "files": files,
        });

        serde_json::to_string_pretty(&combined).map_err(|e| e.to_string())
    }

    #[tool(description = "Get the diff content for a specific file in a review")]
    async fn get_diff(
        &self,
        Parameters(input): Parameters<GetDiffInput>,
    ) -> Result<String, String> {
        let encoded_path = urlencoding::encode(&input.file_path);
        let diff: serde_json::Value = self
            .client
            .get(&format!(
                "/api/reviews/{}/files/{encoded_path}",
                input.review_id
            ))
            .await
            .map_err(format_error)?;

        serde_json::to_string_pretty(&diff).map_err(|e| e.to_string())
    }

    #[tool(description = "Get comment threads on a review, optionally filtered by file path")]
    async fn get_comments(
        &self,
        Parameters(input): Parameters<GetCommentsInput>,
    ) -> Result<String, String> {
        let path = match &input.file_path {
            Some(file) => format!(
                "/api/reviews/{}/threads?file={}",
                input.review_id,
                urlencoding::encode(file)
            ),
            None => format!("/api/reviews/{}/threads", input.review_id),
        };

        let threads: serde_json::Value = self.client.get(&path).await.map_err(format_error)?;

        serde_json::to_string_pretty(&threads).map_err(|e| e.to_string())
    }

    #[tool(description = "Reply to a comment thread as the AI agent")]
    async fn respond_to_comment(
        &self,
        Parameters(input): Parameters<RespondToCommentInput>,
    ) -> Result<String, String> {
        let body = serde_json::json!({
            "author_type": "Agent",
            "body": input.body,
        });

        let comment: serde_json::Value = self
            .client
            .post(&format!("/api/threads/{}/comments", input.thread_id), &body)
            .await
            .map_err(format_error)?;

        serde_json::to_string_pretty(&comment).map_err(|e| e.to_string())
    }

    #[tool(
        description = "Submit a new revision after making code changes in response to review feedback"
    )]
    async fn submit_revision(
        &self,
        Parameters(input): Parameters<SubmitRevisionInput>,
    ) -> Result<String, String> {
        let body = serde_json::json!({
            "trigger": "Agent",
            "message": input.message,
        });

        let revision: serde_json::Value = self
            .client
            .post(
                &format!("/api/reviews/{}/revisions", input.review_id),
                &body,
            )
            .await
            .map_err(format_error)?;

        serde_json::to_string_pretty(&revision).map_err(|e| e.to_string())
    }

    #[tool(description = "Create a new code review from a git repository's uncommitted changes")]
    async fn create_review(
        &self,
        Parameters(input): Parameters<CreateReviewInput>,
    ) -> Result<String, String> {
        let body = serde_json::json!({
            "repo_path": input.repo_path,
            "title": input.title,
            "base_ref": input.base_ref.unwrap_or_else(|| "HEAD".to_string()),
        });

        let review: serde_json::Value = self
            .client
            .post("/api/reviews", &body)
            .await
            .map_err(format_error)?;

        serde_json::to_string_pretty(&review).map_err(|e| e.to_string())
    }

    #[tool(
        description = "Create a new comment thread on a file. Use origin 'AgentExplanation' to proactively explain code."
    )]
    async fn create_thread(
        &self,
        Parameters(input): Parameters<CreateThreadInput>,
    ) -> Result<String, String> {
        let origin = input.origin.unwrap_or_else(|| "Comment".to_string());
        let body = serde_json::json!({
            "file_path": input.file_path,
            "line_start": input.line_start,
            "line_end": input.line_end,
            "origin": origin,
            "body": input.body,
            "author_type": "Agent",
        });

        let thread: serde_json::Value = self
            .client
            .post(&format!("/api/reviews/{}/threads", input.review_id), &body)
            .await
            .map_err(format_error)?;

        serde_json::to_string_pretty(&thread).map_err(|e| e.to_string())
    }

    #[tool(description = "Update a review's status (open or close it)")]
    async fn update_review_status(
        &self,
        Parameters(input): Parameters<UpdateReviewStatusInput>,
    ) -> Result<String, String> {
        let body = serde_json::json!({ "status": input.status });

        self.client
            .patch(&format!("/api/reviews/{}/status", input.review_id), &body)
            .await
            .map_err(format_error)?;

        Ok(format!(
            "Review {} status updated to {}",
            input.review_id, input.status
        ))
    }

    #[tool(description = "Resolve or reopen a comment thread")]
    async fn resolve_thread(
        &self,
        Parameters(input): Parameters<ResolveThreadInput>,
    ) -> Result<String, String> {
        let body = serde_json::json!({ "status": input.status });

        self.client
            .patch(&format!("/api/threads/{}/status", input.thread_id), &body)
            .await
            .map_err(format_error)?;

        Ok(format!(
            "Thread {} status updated to {}",
            input.thread_id, input.status
        ))
    }

    #[tool(
        description = "Wait for a real-time event (new comment, thread created, etc). Blocks until a matching event arrives or timeout. Use this from a background task to monitor a review for activity."
    )]
    async fn wait_for_event(
        &self,
        Parameters(input): Parameters<WaitForEventInput>,
    ) -> Result<String, String> {
        let timeout_secs = input.timeout_secs.unwrap_or(300).min(600);
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let mut rx = self.ws_tx.subscribe();

        let result = tokio::time::timeout(timeout, async {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        // Filter by review_id if specified
                        if let Some(ref rid) = input.review_id
                            && &event.review_id != rid
                        {
                            continue;
                        }
                        // Filter by event type if specified
                        if let Some(ref types) = input.event_types
                            && !types
                                .iter()
                                .any(|t| event_type_matches(&event.event_type, t))
                        {
                            continue;
                        }
                        return Ok(event);
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!("[mcp] wait_for_event: skipped {n} events (lagged)");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return Err("Event channel closed".to_string());
                    }
                }
            }
        })
        .await;

        match result {
            Ok(Ok(event)) => {
                let output = serde_json::json!({
                    "event_type": event.event_type,
                    "review_id": event.review_id,
                    "payload": event.payload,
                    "timestamp": event.timestamp,
                });
                serde_json::to_string_pretty(&output).map_err(|e| e.to_string())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                let output = serde_json::json!({
                    "timeout": true,
                    "message": format!("No matching events within {timeout_secs}s"),
                });
                serde_json::to_string_pretty(&output).map_err(|e| e.to_string())
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for PreflightMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "preflight".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            instructions: Some(
                "Preflight is a local code review tool. Use these tools to participate in code reviews.\n\n\
                 Core loop: list_reviews → get_review → get_diff → get_comments → respond_to_comment\n\n\
                 Agent actions: create_review (start a review), create_thread (comment on code or explain it \
                 with origin 'AgentExplanation'), submit_revision (after making changes)\n\n\
                 Lifecycle: update_review_status (open/close), resolve_thread (resolve/reopen)\n\n\
                 Notifications: Use wait_for_event from a background task to monitor for new comments, \
                 threads, or status changes. It blocks until a matching event arrives or times out."
                    .to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use preflight_core::ws::{WsEvent, WsEventType};
    use rmcp::handler::server::wrapper::Parameters;

    fn test_mcp() -> PreflightMcp {
        let client = crate::client::PreflightClient::new(19999); // dummy port
        let (ws_tx, _) = broadcast::channel(64);
        PreflightMcp::new(client, ws_tx)
    }

    #[tokio::test]
    async fn wait_for_event_receives_matching_event() {
        let mcp = test_mcp();
        let ws_tx = mcp.ws_tx.clone();

        // Spawn event sender after a short delay
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let _ = ws_tx.send(WsEvent {
                event_type: WsEventType::CommentAdded,
                review_id: "test-review".to_string(),
                payload: serde_json::json!({"thread_id": "t1"}),
                timestamp: chrono::Utc::now(),
            });
        });

        let result = mcp
            .wait_for_event(Parameters(WaitForEventInput {
                review_id: Some("test-review".to_string()),
                event_types: Some(vec!["comment_added".to_string()]),
                timeout_secs: Some(5),
            }))
            .await
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["event_type"], "comment_added");
        assert_eq!(parsed["review_id"], "test-review");
    }

    #[tokio::test]
    async fn wait_for_event_times_out() {
        let mcp = test_mcp();

        let result = mcp
            .wait_for_event(Parameters(WaitForEventInput {
                review_id: None,
                event_types: None,
                timeout_secs: Some(1),
            }))
            .await
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["timeout"], true);
    }

    #[tokio::test]
    async fn wait_for_event_filters_by_review_id() {
        let mcp = test_mcp();
        let ws_tx = mcp.ws_tx.clone();

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            // Send event for wrong review
            let _ = ws_tx.send(WsEvent {
                event_type: WsEventType::CommentAdded,
                review_id: "other-review".to_string(),
                payload: serde_json::json!({}),
                timestamp: chrono::Utc::now(),
            });
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            // Send event for correct review
            let _ = ws_tx.send(WsEvent {
                event_type: WsEventType::CommentAdded,
                review_id: "my-review".to_string(),
                payload: serde_json::json!({"thread_id": "t2"}),
                timestamp: chrono::Utc::now(),
            });
        });

        let result = mcp
            .wait_for_event(Parameters(WaitForEventInput {
                review_id: Some("my-review".to_string()),
                event_types: None,
                timeout_secs: Some(5),
            }))
            .await
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["review_id"], "my-review");
    }

    #[tokio::test]
    async fn wait_for_event_filters_by_event_type() {
        let mcp = test_mcp();
        let ws_tx = mcp.ws_tx.clone();

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            // Send non-matching event type
            let _ = ws_tx.send(WsEvent {
                event_type: WsEventType::ReviewStatusChanged,
                review_id: "r1".to_string(),
                payload: serde_json::json!({}),
                timestamp: chrono::Utc::now(),
            });
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            // Send matching event type
            let _ = ws_tx.send(WsEvent {
                event_type: WsEventType::ThreadCreated,
                review_id: "r1".to_string(),
                payload: serde_json::json!({"thread_id": "t3"}),
                timestamp: chrono::Utc::now(),
            });
        });

        let result = mcp
            .wait_for_event(Parameters(WaitForEventInput {
                review_id: None,
                event_types: Some(vec!["thread_created".to_string()]),
                timeout_secs: Some(5),
            }))
            .await
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["event_type"], "thread_created");
    }
}
