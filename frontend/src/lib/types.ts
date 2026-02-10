// --- Enums (match Rust serde default: PascalCase variant names) ---

export type ReviewStatus = "Open" | "Closed";
export type FileStatus =
  | "Added"
  | "Modified"
  | "Deleted"
  | "Renamed"
  | "Binary";
export type ThreadOrigin =
  | "Comment"
  | "ExplanationRequest"
  | "AgentExplanation";
export type ThreadStatus = "Open" | "Resolved";
export type AgentStatus = "Seen" | "Working";
export type AuthorType = "Human" | "Agent";
export type LineKind = "Context" | "Added" | "Removed";
export type RevisionTrigger = "Agent" | "Manual";

// --- Response types ---

export interface ReviewResponse {
  id: string;
  title: string | null;
  status: ReviewStatus;
  file_count: number;
  thread_count: number;
  open_thread_count: number;
  revision_count: number;
  created_at: string;
  updated_at: string;
}

export interface RevisionResponse {
  id: string;
  review_id: string;
  revision_number: number;
  trigger: RevisionTrigger;
  message: string | null;
  file_count: number;
  created_at: string;
}

export interface FileListEntry {
  path: string;
  status: FileStatus;
  thread_count: number;
  open_thread_count: number;
}

export interface FileDiffResponse {
  path: string;
  old_path: string | null;
  status: FileStatus;
  hunks: Hunk[];
}

export interface Hunk {
  old_start: number;
  old_count: number;
  new_start: number;
  new_count: number;
  context: string | null;
  lines: DiffLine[];
}

export interface DiffLine {
  kind: LineKind;
  content: string;
  old_line_no: number | null;
  new_line_no: number | null;
  highlighted?: string;
}

export interface FileContentLine {
  line_no: number;
  content: string;
  highlighted?: string;
}

export interface FileContentResponse {
  path: string;
  language: string | null;
  lines: FileContentLine[];
}

export interface ThreadResponse {
  id: string;
  review_id: string;
  file_path: string;
  line_start: number;
  line_end: number;
  origin: ThreadOrigin;
  status: ThreadStatus;
  agent_status: AgentStatus | null;
  comments: CommentResponse[];
  created_at: string;
  updated_at: string;
}

export interface CommentResponse {
  id: string;
  author_type: AuthorType;
  body: string;
  created_at: string;
}

// --- Request types ---

export interface CreateReviewRequest {
  title?: string;
  repo_path: string;
  base_ref: string;
}

export interface CreateRevisionRequest {
  trigger: RevisionTrigger;
  message?: string;
}

export interface UpdateReviewStatusRequest {
  status: ReviewStatus;
}

export interface CreateThreadRequest {
  file_path: string;
  line_start: number;
  line_end: number;
  origin: ThreadOrigin;
  body: string;
  author_type: AuthorType;
}

export interface UpdateThreadStatusRequest {
  status: ThreadStatus;
}

export interface AddCommentRequest {
  author_type: AuthorType;
  body: string;
}

// --- WebSocket events ---

export type WsEventType =
  | "review_created"
  | "review_status_changed"
  | "review_deleted"
  | "revision_created"
  | "thread_created"
  | "comment_added"
  | "thread_status_changed"
  | "thread_acknowledged"
  | "thread_poked"
  | "revision_requested"
  | "agent_presence_changed";

export interface AgentPresenceResponse {
  connected: boolean;
}

export interface WsEvent {
  event_type: WsEventType;
  review_id: string;
  payload: unknown;
  timestamp: string;
}
