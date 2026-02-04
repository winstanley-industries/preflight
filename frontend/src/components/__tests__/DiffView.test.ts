import { render, screen, cleanup } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { FileDiffResponse, ThreadResponse } from "../../lib/types";
import DiffView from "../DiffView.svelte";

vi.mock("../../lib/api", () => ({
  getFileDiff: vi.fn(),
  getFileContent: vi.fn(),
  createThread: vi.fn(),
}));

import { getFileDiff, createThread } from "../../lib/api";
const mockGetFileDiff = vi.mocked(getFileDiff);
const mockCreateThread = vi.mocked(createThread);

const FIXTURE: FileDiffResponse = {
  path: "src/main.ts",
  old_path: null,
  status: "Modified",
  hunks: [
    {
      old_start: 1,
      old_count: 3,
      new_start: 1,
      new_count: 4,
      context: "function main()",
      lines: [
        {
          kind: "Context",
          content: "import { foo } from 'bar';",
          old_line_no: 1,
          new_line_no: 1,
        },
        {
          kind: "Removed",
          content: "const x = 1;",
          old_line_no: 2,
          new_line_no: null,
        },
        {
          kind: "Added",
          content: "const x = 2;",
          old_line_no: null,
          new_line_no: 2,
        },
        {
          kind: "Added",
          content: "const y = 3;",
          old_line_no: null,
          new_line_no: 3,
        },
        {
          kind: "Context",
          content: "export default x;",
          old_line_no: 3,
          new_line_no: 4,
        },
      ],
    },
    {
      old_start: 10,
      old_count: 1,
      new_start: 11,
      new_count: 1,
      context: null,
      lines: [
        {
          kind: "Context",
          content: "// end",
          old_line_no: 10,
          new_line_no: 11,
        },
      ],
    },
  ],
};

const THREAD_ON_LINE_2: ThreadResponse = {
  id: "t-1",
  review_id: "rev-1",
  file_path: "src/main.ts",
  line_start: 2,
  line_end: 2,
  origin: "Comment",
  status: "Open",
  comments: [{ id: "c-1", author_type: "Human", body: "Why?", created_at: "" }],
  created_at: "",
  updated_at: "",
};

async function renderDiff(
  threads: ThreadResponse[] = [],
  overrides: Record<string, unknown> = {},
) {
  mockGetFileDiff.mockResolvedValueOnce(FIXTURE);
  const result = render(DiffView, {
    props: {
      reviewId: "rev-1",
      filePath: "src/main.ts",
      fileStatus: "Modified" as const,
      hasRepoPath: true,
      threads,
      ...overrides,
    },
  });
  // Wait for async load
  await screen.findByText("function main()");
  return result;
}

describe("DiffView", () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders hunk headers and diff lines", async () => {
    await renderDiff();
    // Hunk header with context
    expect(screen.getByText("function main()")).toBeInTheDocument();
    // Line content
    expect(screen.getByText("import { foo } from 'bar';")).toBeInTheDocument();
    expect(screen.getByText("const x = 2;")).toBeInTheDocument();
    expect(screen.getByText("// end")).toBeInTheDocument();
  });

  it('shows "+" button on commentable lines (those with new_line_no)', async () => {
    await renderDiff();
    // Lines with new_line_no get a button; count them
    // Context line 1, Added lines 2 & 3, Context line 4, Context line 11 = 5 commentable
    // Removed line (old_line_no=2, new_line_no=null) should NOT have a button
    // +2 for the Diff/File toggle buttons at the top
    const buttons = screen.getAllByRole("button");
    expect(buttons.length).toBe(7);
  });

  it("no + on removed-only lines (null new_line_no)", async () => {
    await renderDiff();
    // The removed line "const x = 1;" should not have a sibling button
    const removedLine = screen.getByText("const x = 1;");
    const row = removedLine.closest(".flex");
    const btn = row?.querySelector("button");
    expect(btn).toBeNull();
  });

  it('clicking "+" opens InlineCommentForm below that line', async () => {
    const user = userEvent.setup();
    await renderDiff();
    const buttons = screen.getAllByRole("button");
    // Click the first commentable line's gutter button (line 1)
    // buttons[0] and buttons[1] are the Diff/File toggle buttons
    await user.click(buttons[2]);
    // The inline form should appear with a textarea
    expect(screen.getByPlaceholderText("Add a comment...")).toBeInTheDocument();
  });

  it("shift-clicking extends selection range", async () => {
    const user = userEvent.setup();
    await renderDiff();
    const buttons = screen.getAllByRole("button");
    // Click line 2 (buttons[3] after 2 toggle buttons, first Added line)
    await user.click(buttons[3]);
    // Shift-click line 3 (buttons[4], second Added line)
    await user.keyboard("{Shift>}");
    await user.click(buttons[4]);
    await user.keyboard("{/Shift}");
    // Form should show "Lines 2–3"
    expect(screen.getByText("Lines 2\u20133")).toBeInTheDocument();
  });

  it("Escape closes the form and clears selection", async () => {
    const user = userEvent.setup();
    await renderDiff();
    const buttons = screen.getAllByRole("button");
    // buttons[2] is the first gutter button (after 2 toggle buttons)
    await user.click(buttons[2]);
    // Form is open
    expect(screen.getByPlaceholderText("Add a comment...")).toBeInTheDocument();
    // Press Escape
    await user.keyboard("{Escape}");
    expect(
      screen.queryByPlaceholderText("Add a comment..."),
    ).not.toBeInTheDocument();
  });

  it("shows thread dots on lines that have threads", async () => {
    await renderDiff([THREAD_ON_LINE_2]);
    // Line 2 has a thread — the bullet should be visible
    const bulletContainer = document.getElementById("L2");
    expect(bulletContainer).not.toBeNull();
    const bullet = bulletContainer?.querySelector("button .text-accent");
    expect(bullet?.textContent?.trim()).toBe("•");
  });

  it("renders highlighted HTML when present", async () => {
    const highlightedFixture: FileDiffResponse = {
      path: "src/main.ts",
      old_path: null,
      status: "Modified",
      hunks: [
        {
          old_start: 1,
          old_count: 1,
          new_start: 1,
          new_count: 1,
          context: null,
          lines: [
            {
              kind: "Context",
              content: "const x = 1;",
              old_line_no: 1,
              new_line_no: 1,
              highlighted: '<span class="sy-keyword">const</span> x = 1;',
            },
          ],
        },
      ],
    };
    mockGetFileDiff.mockResolvedValueOnce(highlightedFixture);
    render(DiffView, {
      props: {
        reviewId: "rev-1",
        filePath: "src/main.ts",
        fileStatus: "Modified" as const,
        hasRepoPath: true,
        threads: [],
      },
    });
    await screen.findByText("const", { selector: ".sy-keyword" });
  });

  it("calls onThreadCreated after form submission", async () => {
    const user = userEvent.setup();
    const onThreadCreated = vi.fn();
    mockCreateThread.mockResolvedValueOnce({
      id: "thread-new",
      review_id: "rev-1",
      file_path: "src/main.ts",
      line_start: 1,
      line_end: 1,
      origin: "Comment",
      status: "Open",
      comments: [],
      created_at: "",
      updated_at: "",
    });
    await renderDiff([], { onThreadCreated });
    const buttons = screen.getAllByRole("button");
    // buttons[2] is the first gutter button (after 2 toggle buttons)
    await user.click(buttons[2]);
    await user.type(screen.getByRole("textbox"), "Nice change");
    await user.click(screen.getByRole("button", { name: "Submit" }));
    expect(onThreadCreated).toHaveBeenCalledWith("thread-new");
  });
});
