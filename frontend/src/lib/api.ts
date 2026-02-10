import type {
  AddCommentRequest,
  AgentPresenceResponse,
  AgentStatus,
  CommentResponse,
  CreateReviewRequest,
  CreateRevisionRequest,
  CreateThreadRequest,
  FileContentResponse,
  FileDiffResponse,
  FileListEntry,
  ReviewResponse,
  RevisionResponse,
  ThreadResponse,
  UpdateReviewStatusRequest,
  UpdateThreadStatusRequest,
} from "./types";

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }));
    throw new ApiError(res.status, body.error ?? res.statusText);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

// --- Reviews ---

export function listReviews(): Promise<ReviewResponse[]> {
  return request("/api/reviews");
}

export function getReview(id: string): Promise<ReviewResponse> {
  return request(`/api/reviews/${id}`);
}

export function createReview(
  req: CreateReviewRequest,
): Promise<ReviewResponse> {
  return request("/api/reviews", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export function updateReviewStatus(
  id: string,
  req: UpdateReviewStatusRequest,
): Promise<void> {
  return request(`/api/reviews/${id}/status`, {
    method: "PATCH",
    body: JSON.stringify(req),
  });
}

export function deleteReview(id: string): Promise<void> {
  return request(`/api/reviews/${id}`, { method: "DELETE" });
}

export function deleteClosedReviews(): Promise<void> {
  return request("/api/reviews", { method: "DELETE" });
}

// --- Revisions ---

export function listRevisions(reviewId: string): Promise<RevisionResponse[]> {
  return request(`/api/reviews/${reviewId}/revisions`);
}

export function createRevision(
  reviewId: string,
  req: CreateRevisionRequest,
): Promise<RevisionResponse> {
  return request(`/api/reviews/${reviewId}/revisions`, {
    method: "POST",
    body: JSON.stringify(req),
  });
}

// --- Files ---

export function listFiles(
  reviewId: string,
  revision?: number,
): Promise<FileListEntry[]> {
  const params = revision != null ? `?revision=${revision}` : "";
  return request(`/api/reviews/${reviewId}/files${params}`);
}

export function getFileDiff(
  reviewId: string,
  path: string,
  revision?: number,
): Promise<FileDiffResponse> {
  const params = revision != null ? `?revision=${revision}` : "";
  return request(`/api/reviews/${reviewId}/files/${path}${params}`);
}

export function getFileInterdiff(
  reviewId: string,
  path: string,
  from: number,
  to: number,
): Promise<FileDiffResponse> {
  return request(
    `/api/reviews/${reviewId}/interdiff/${path}?from=${from}&to=${to}`,
  );
}

export function getFileContent(
  reviewId: string,
  path: string,
  version?: "old" | "new",
): Promise<FileContentResponse> {
  const params = version ? `?version=${version}` : "";
  return request(`/api/reviews/${reviewId}/content/${path}${params}`);
}

// --- Threads ---

export function listThreads(
  reviewId: string,
  filePath?: string,
): Promise<ThreadResponse[]> {
  const params = filePath ? `?file=${encodeURIComponent(filePath)}` : "";
  return request(`/api/reviews/${reviewId}/threads${params}`);
}

export function createThread(
  reviewId: string,
  req: CreateThreadRequest,
): Promise<ThreadResponse> {
  return request(`/api/reviews/${reviewId}/threads`, {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export function updateThreadStatus(
  threadId: string,
  req: UpdateThreadStatusRequest,
): Promise<void> {
  return request(`/api/threads/${threadId}/status`, {
    method: "PATCH",
    body: JSON.stringify(req),
  });
}

export function setAgentStatus(
  threadId: string,
  status: AgentStatus,
): Promise<void> {
  return request(`/api/threads/${threadId}/agent-status`, {
    method: "PUT",
    body: JSON.stringify({ status }),
  });
}

export function pokeThread(threadId: string): Promise<void> {
  return request(`/api/threads/${threadId}/poke`, {
    method: "POST",
  });
}

// --- Comments ---

export function addComment(
  threadId: string,
  req: AddCommentRequest,
): Promise<CommentResponse> {
  return request(`/api/threads/${threadId}/comments`, {
    method: "POST",
    body: JSON.stringify(req),
  });
}

// --- Agent ---

export function getAgentPresence(
  reviewId: string,
): Promise<AgentPresenceResponse> {
  return request(`/api/reviews/${reviewId}/agent-status`);
}

export function requestRevision(reviewId: string): Promise<void> {
  return request(`/api/reviews/${reviewId}/request-revision`, {
    method: "POST",
  });
}

// --- Health ---

export async function healthCheck(): Promise<{
  status: string;
  version: string;
}> {
  return request("/api/health");
}
