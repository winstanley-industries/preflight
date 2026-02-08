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
  agent_status: null,
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
      diffLines: new Set([5, 6, 7, 8]),
      onThreadsChanged: vi.fn(),
      onNavigateToThread: vi.fn(),
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

  it('clicking "Lines X–Y" calls onNavigateToThread with line_start', async () => {
    const user = userEvent.setup();
    const onNavigateToThread = vi.fn();
    renderPanel([OPEN_THREAD], { onNavigateToThread });
    await user.click(screen.getByText(/Lines 5/));
    expect(onNavigateToThread).toHaveBeenCalledWith(5);
  });

  it("threads in diff show accent color, threads outside diff show muted color with arrow", () => {
    const outsideThread: ThreadResponse = {
      ...OPEN_THREAD,
      id: "t-outside",
      line_start: 20,
      line_end: 22,
    };
    // diffLines contains 5-8, so OPEN_THREAD (5-8) is in diff, outsideThread (20-22) is not
    renderPanel([OPEN_THREAD, outsideThread]);

    const buttons = screen.getAllByRole("button", { name: /Lines/ });
    // In-diff thread should have accent color
    expect(buttons[0].className).toContain("text-accent");
    expect(buttons[0].className).not.toContain("text-text-muted");
    // Out-of-diff thread should have muted color and arrow
    expect(buttons[1].className).toContain("text-text-muted");
    expect(buttons[1].textContent).toContain("→");
  });

  it('threads outside diff have "Opens full file view" tooltip', () => {
    const outsideThread: ThreadResponse = {
      ...OPEN_THREAD,
      id: "t-outside",
      line_start: 20,
      line_end: 22,
    };
    renderPanel([outsideThread]);
    const button = screen.getByRole("button", { name: /Lines 20/ });
    expect(button.title).toBe("Opens full file view");
  });
});
