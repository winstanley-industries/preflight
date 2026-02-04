import { render, screen, cleanup } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ThreadResponse } from "../../lib/types";
import ThreadPanel from "../ThreadPanel.svelte";

vi.mock("../../lib/api", () => ({
  addComment: vi.fn(),
  updateThreadStatus: vi.fn(),
}));

import { addComment, updateThreadStatus } from "../../lib/api";
const mockAddComment = vi.mocked(addComment);
const mockUpdateThreadStatus = vi.mocked(updateThreadStatus);

const OPEN_THREAD: ThreadResponse = {
  id: "t-1",
  review_id: "rev-1",
  file_path: "src/main.ts",
  line_start: 5,
  line_end: 8,
  origin: "Comment",
  status: "Open",
  comments: [
    { id: "c-1", author_type: "Human", body: "Looks wrong", created_at: "" },
    { id: "c-2", author_type: "Agent", body: "Will fix", created_at: "" },
  ],
  created_at: "",
  updated_at: "",
};

const RESOLVED_THREAD: ThreadResponse = {
  ...OPEN_THREAD,
  id: "t-2",
  status: "Resolved",
  origin: "ExplanationRequest",
  comments: [
    { id: "c-3", author_type: "Human", body: "Explain this", created_at: "" },
  ],
};

function renderPanel(
  threads: ThreadResponse[] = [],
  overrides: Record<string, unknown> = {},
) {
  return render(ThreadPanel, {
    props: {
      threads,
      highlightThreadId: null,
      onThreadsChanged: vi.fn(),
      ...overrides,
    },
  });
}

describe("ThreadPanel", () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('renders "No threads on this file." when empty', () => {
    renderPanel([]);
    expect(screen.getByText("No threads on this file.")).toBeInTheDocument();
  });

  it("renders thread with comments, origin badge, status badge", () => {
    renderPanel([OPEN_THREAD]);
    expect(screen.getByText("Looks wrong")).toBeInTheDocument();
    expect(screen.getByText("Will fix")).toBeInTheDocument();
    expect(screen.getByText("Comment")).toBeInTheDocument();
    expect(screen.getByText("Open")).toBeInTheDocument();
  });

  it("reply textarea submits with Cmd+Enter and calls addComment", async () => {
    const user = userEvent.setup();
    const onThreadsChanged = vi.fn();
    mockAddComment.mockResolvedValueOnce({
      id: "c-new",
      author_type: "Human",
      body: "Thanks",
      created_at: "",
    });
    renderPanel([OPEN_THREAD], { onThreadsChanged });
    const textarea = screen.getByPlaceholderText("Reply...");
    await user.type(textarea, "Thanks");
    await user.keyboard("{Meta>}{Enter}{/Meta}");
    expect(mockAddComment).toHaveBeenCalledWith("t-1", {
      author_type: "Human",
      body: "Thanks",
    });
    expect(onThreadsChanged).toHaveBeenCalled();
  });

  it('"Resolve" button calls updateThreadStatus with "Resolved"', async () => {
    const user = userEvent.setup();
    const onThreadsChanged = vi.fn();
    mockUpdateThreadStatus.mockResolvedValueOnce(undefined);
    renderPanel([OPEN_THREAD], { onThreadsChanged });
    await user.click(screen.getByRole("button", { name: "Resolve" }));
    expect(mockUpdateThreadStatus).toHaveBeenCalledWith("t-1", {
      status: "Resolved",
    });
    expect(onThreadsChanged).toHaveBeenCalled();
  });

  it('"Reopen" button appears on resolved threads', () => {
    renderPanel([RESOLVED_THREAD]);
    expect(screen.getByRole("button", { name: "Reopen" })).toBeInTheDocument();
  });

  it('clicking "Lines X–Y" calls scrollIntoView', async () => {
    const user = userEvent.setup();
    // Create a target element for scrollIntoView
    const target = document.createElement("div");
    target.id = "L5";
    target.scrollIntoView = vi.fn();
    document.body.appendChild(target);

    renderPanel([OPEN_THREAD]);
    await user.click(screen.getByText(/Lines 5/));
    expect(target.scrollIntoView).toHaveBeenCalledWith({
      behavior: "smooth",
      block: "center",
    });

    document.body.removeChild(target);
  });
});
