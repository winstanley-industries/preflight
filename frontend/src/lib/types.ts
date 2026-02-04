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
export type AuthorType = "Human" | "Agent";
export type LineKind = "Context" | "Added" | "Removed";

// --- Response types ---

export interface ReviewResponse {
  id: string;
  title: string | null;
  status: ReviewStatus;
  file_count: number;
  thread_count: number;
  created_at: string;
  updated_at: string;
}

export interface FileListEntry {
  path: string;
  status: FileStatus;
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

export interface ThreadResponse {
  id: string;
  review_id: string;
  file_path: string;
  line_start: number;
  line_end: number;
  origin: ThreadOrigin;
  status: ThreadStatus;
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
  diff: string;
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
