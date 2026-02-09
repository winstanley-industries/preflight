import { render, screen, cleanup } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ThreadResponse } from "../../lib/types";
import ThreadPanel from "../ThreadPanel.svelte";

vi.mock("../../lib/api", () => ({
  addComment: vi.fn(),
  updateThreadStatus: vi.fn(),
  pokeThread: vi.fn(),
}));

import { addComment, updateThreadStatus, pokeThread } from "../../lib/api";
const mockAddComment = vi.mocked(addComment);
const mockUpdateThreadStatus = vi.mocked(updateThreadStatus);
const mockPokeThread = vi.mocked(pokeThread);

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
  created_at: "2026-02-09T01:00:00Z",
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
  created_at: "2026-02-09T03:00:00Z",
};

const AGENT_EXPLANATION_THREAD: ThreadResponse = {
  ...OPEN_THREAD,
  id: "t-3",
  origin: "AgentExplanation",
  line_start: 15,
  line_end: 18,
  comments: [
    {
      id: "c-4",
      author_type: "Agent",
      body: "This function handles auth",
      created_at: "",
    },
  ],
  created_at: "2026-02-09T02:00:00Z",
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
    // Origin badge (not the dropdown option)
    const originBadges = screen.getAllByText("Comment");
    expect(originBadges.length).toBeGreaterThanOrEqual(1);
    // Status badge in thread header
    const openBadges = screen.getAllByText("Open");
    expect(openBadges.length).toBeGreaterThanOrEqual(1);
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

  it('"Reopen" button appears on resolved threads', async () => {
    const user = userEvent.setup();
    renderPanel([RESOLVED_THREAD]);
    await user.click(screen.getByRole("button", { name: /Resolved\s+1/ }));
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

  // --- Agent activity indicator tests ---

  it('shows "Agent has seen this" when agent_status is Seen', () => {
    const thread: ThreadResponse = {
      ...OPEN_THREAD,
      agent_status: "Seen",
    };
    renderPanel([thread]);
    expect(screen.getByText("Agent has seen this")).toBeInTheDocument();
    expect(screen.queryByText(/working/i)).not.toBeInTheDocument();
    expect(screen.queryByText("Nudge agent")).not.toBeInTheDocument();
  });

  it('shows "Agent is working on this" with spinner when agent_status is Working', () => {
    const thread: ThreadResponse = {
      ...OPEN_THREAD,
      agent_status: "Working",
    };
    const { container } = renderPanel([thread]);
    // The text contains &hellip; which renders as "…"
    const indicator = container.querySelector(".animate-spin")?.parentElement;
    expect(indicator).toBeTruthy();
    expect(indicator!.textContent).toContain("Agent is working on this");
    expect(screen.queryByText("Agent has seen this")).not.toBeInTheDocument();
    expect(screen.queryByText("Nudge agent")).not.toBeInTheDocument();
  });

  it('shows "Nudge agent" button when no agent_status, thread is open, last comment is human', () => {
    const thread: ThreadResponse = {
      ...OPEN_THREAD,
      agent_status: null,
      comments: [
        {
          id: "c-1",
          author_type: "Human",
          body: "Please fix this",
          created_at: "",
        },
      ],
    };
    renderPanel([thread]);
    expect(
      screen.getByRole("button", { name: "Nudge agent" }),
    ).toBeInTheDocument();
  });

  it("does not show nudge button when last comment is from agent", () => {
    // OPEN_THREAD has last comment from Agent
    renderPanel([OPEN_THREAD]);
    expect(screen.queryByText("Nudge agent")).not.toBeInTheDocument();
  });

  it("does not show nudge button on resolved threads", async () => {
    const user = userEvent.setup();
    const thread: ThreadResponse = {
      ...RESOLVED_THREAD,
      agent_status: null,
    };
    renderPanel([thread]);
    await user.click(screen.getByRole("button", { name: /Resolved\s+1/ }));
    expect(screen.queryByText("Nudge agent")).not.toBeInTheDocument();
  });

  it("nudge button calls pokeThread and shows feedback", async () => {
    const user = userEvent.setup();
    mockPokeThread.mockResolvedValueOnce(undefined);
    const thread: ThreadResponse = {
      ...OPEN_THREAD,
      agent_status: null,
      comments: [
        {
          id: "c-1",
          author_type: "Human",
          body: "Hello?",
          created_at: "",
        },
      ],
    };
    renderPanel([thread]);
    await user.click(screen.getByRole("button", { name: "Nudge agent" }));
    expect(mockPokeThread).toHaveBeenCalledWith("t-1");
    expect(screen.getByText("Nudged!")).toBeInTheDocument();
  });

  it("shows no agent indicator when agent_status is null and conditions for nudge are not met", () => {
    // OPEN_THREAD: agent_status null, last comment is Agent → no indicator shown
    renderPanel([OPEN_THREAD]);
    expect(screen.queryByText("Agent has seen this")).not.toBeInTheDocument();
    expect(screen.queryByText(/working/i)).not.toBeInTheDocument();
    expect(screen.queryByText("Nudge agent")).not.toBeInTheDocument();
  });

  // --- Filtering and sorting tests ---

  describe("filtering and sorting", () => {
    const ALL_THREADS = [
      OPEN_THREAD,
      RESOLVED_THREAD,
      AGENT_EXPLANATION_THREAD,
    ];

    it("defaults to Open filter with status tab counts", () => {
      renderPanel(ALL_THREADS);
      // Only open threads visible by default
      expect(screen.getByText("Looks wrong")).toBeInTheDocument();
      expect(
        screen.getByText("This function handles auth"),
      ).toBeInTheDocument();
      // Resolved thread hidden
      expect(screen.queryByText("Explain this")).not.toBeInTheDocument();
      // Status tabs with counts
      expect(
        screen.getByRole("button", { name: /Open\s+2/ }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /Resolved\s+1/ }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: /All\s+3/ }),
      ).toBeInTheDocument();
    });

    it("shows all threads when clicking All tab", async () => {
      const user = userEvent.setup();
      renderPanel(ALL_THREADS);
      await user.click(screen.getByRole("button", { name: /All\s+3/ }));
      expect(screen.getByText("Looks wrong")).toBeInTheDocument();
      expect(screen.getByText("Explain this")).toBeInTheDocument();
      expect(
        screen.getByText("This function handles auth"),
      ).toBeInTheDocument();
    });

    it("filters by status when clicking Resolved tab", async () => {
      const user = userEvent.setup();
      renderPanel(ALL_THREADS);
      await user.click(screen.getByRole("button", { name: /Resolved\s+1/ }));
      // Resolved thread visible
      expect(screen.getByText("Explain this")).toBeInTheDocument();
      // Open threads hidden
      expect(screen.queryByText("Looks wrong")).not.toBeInTheDocument();
      expect(
        screen.queryByText("This function handles auth"),
      ).not.toBeInTheDocument();
    });

    it("filters by origin when selecting from dropdown", async () => {
      const user = userEvent.setup();
      renderPanel(ALL_THREADS);
      const originSelect = screen.getByLabelText("Filter by origin");
      await user.selectOptions(originSelect, "AgentExplanation");
      // Only agent explanation thread visible
      expect(
        screen.getByText("This function handles auth"),
      ).toBeInTheDocument();
      expect(screen.queryByText("Looks wrong")).not.toBeInTheDocument();
      expect(screen.queryByText("Explain this")).not.toBeInTheDocument();
    });

    it("sorts by location (default) — threads ordered by line_start", async () => {
      const user = userEvent.setup();
      renderPanel(ALL_THREADS);
      // Switch to All to see all 3 threads
      await user.click(screen.getByRole("button", { name: /All\s+3/ }));
      const lineButtons = screen.getAllByRole("button", { name: /Lines/ });
      // OPEN_THREAD line 5, RESOLVED_THREAD line 5, AGENT_EXPLANATION line 15
      expect(lineButtons[0].textContent).toContain("5");
      expect(lineButtons[1].textContent).toContain("5");
      expect(lineButtons[2].textContent).toContain("15");
    });

    it("sorts by newest when selected", async () => {
      const user = userEvent.setup();
      renderPanel(ALL_THREADS);
      // Switch to All to see all 3 threads
      await user.click(screen.getByRole("button", { name: /All\s+3/ }));
      const sortSelect = screen.getByLabelText("Sort threads");
      await user.selectOptions(sortSelect, "newest");
      const lineButtons = screen.getAllByRole("button", { name: /Lines/ });
      // RESOLVED (03:00) line 5, AGENT_EXPLANATION (02:00) line 15, OPEN (01:00) line 5
      expect(lineButtons[0].textContent).toMatch(/5/);
      expect(lineButtons[1].textContent).toMatch(/15/);
      expect(lineButtons[2].textContent).toMatch(/5/);
    });

    it("sorts by oldest when selected", async () => {
      const user = userEvent.setup();
      renderPanel(ALL_THREADS);
      // Switch to All to see all 3 threads
      await user.click(screen.getByRole("button", { name: /All\s+3/ }));
      const sortSelect = screen.getByLabelText("Sort threads");
      await user.selectOptions(sortSelect, "oldest");
      const lineButtons = screen.getAllByRole("button", { name: /Lines/ });
      // OPEN (01:00) line 5, AGENT_EXPLANATION (02:00) line 15, RESOLVED (03:00) line 5
      expect(lineButtons[0].textContent).toMatch(/5/);
      expect(lineButtons[1].textContent).toMatch(/15/);
      expect(lineButtons[2].textContent).toMatch(/5/);
    });

    it("combines status and origin filters", async () => {
      const user = userEvent.setup();
      renderPanel(ALL_THREADS);
      // Filter to Open + AgentExplanation
      await user.click(screen.getByRole("button", { name: /Open\s+2/ }));
      const originSelect = screen.getByLabelText("Filter by origin");
      await user.selectOptions(originSelect, "AgentExplanation");
      // Only the open AgentExplanation thread
      expect(
        screen.getByText("This function handles auth"),
      ).toBeInTheDocument();
      expect(screen.queryByText("Looks wrong")).not.toBeInTheDocument();
      expect(screen.queryByText("Explain this")).not.toBeInTheDocument();
    });

    it("shows 'No matching threads.' when filters exclude everything", async () => {
      const user = userEvent.setup();
      renderPanel(ALL_THREADS);
      // Filter to Resolved + AgentExplanation (no threads match)
      await user.click(screen.getByRole("button", { name: /Resolved\s+1/ }));
      const originSelect = screen.getByLabelText("Filter by origin");
      await user.selectOptions(originSelect, "AgentExplanation");
      expect(screen.getByText("No matching threads.")).toBeInTheDocument();
    });

    it("filter bar is not rendered when there are no threads", () => {
      renderPanel([]);
      expect(
        screen.queryByLabelText("Filter by origin"),
      ).not.toBeInTheDocument();
      expect(screen.queryByLabelText("Sort threads")).not.toBeInTheDocument();
    });
  });
});
