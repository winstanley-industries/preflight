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

import { getFileDiff, getFileContent, createThread } from "../../lib/api";
const mockGetFileDiff = vi.mocked(getFileDiff);
const mockGetFileContent = vi.mocked(getFileContent);
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

  // --- File view toggle ---

  it("clicking File button switches to file view and loads content", async () => {
    const user = userEvent.setup();
    mockGetFileContent.mockResolvedValueOnce({
      path: "src/main.ts",
      language: "typescript",
      lines: [
        { line_no: 1, content: "import { foo } from 'bar';" },
        { line_no: 2, content: "const x = 2;" },
        { line_no: 3, content: "const y = 3;" },
      ],
    });
    await renderDiff();
    await user.click(screen.getByRole("button", { name: "File" }));
    // Wait for file content to load — look for a line number from file view
    await screen.findByText("import { foo } from 'bar';");
    expect(mockGetFileContent).toHaveBeenCalledWith(
      "rev-1",
      "src/main.ts",
      "new",
    );
  });

  it("version buttons disable correctly based on fileStatus", async () => {
    const user = userEvent.setup();
    mockGetFileContent.mockResolvedValueOnce({
      path: "src/new.ts",
      language: "typescript",
      lines: [{ line_no: 1, content: "new file" }],
    });
    await renderDiff([], { fileStatus: "Added" as const });
    await user.click(screen.getByRole("button", { name: "File" }));
    await screen.findByText("new file");
    // Old button should be disabled for Added files
    const oldBtn = screen.getByRole("button", { name: "Old" });
    expect(oldBtn).toBeDisabled();
  });

  // --- Changed line highlighting ---

  it("highlights changed lines in file view", async () => {
    const user = userEvent.setup();
    mockGetFileContent.mockResolvedValueOnce({
      path: "src/main.ts",
      language: "typescript",
      lines: [
        { line_no: 1, content: "import { foo } from 'bar';" },
        { line_no: 2, content: "const x = 2;" },
        { line_no: 3, content: "const y = 3;" },
        { line_no: 4, content: "export default x;" },
      ],
    });
    await renderDiff();
    await user.click(screen.getByRole("button", { name: "File" }));
    await screen.findByText("const x = 2;");

    // Lines 2 and 3 are Added in the diff — they should have the add highlight
    const line2 = document.getElementById("L2");
    const line3 = document.getElementById("L3");
    const line1 = document.getElementById("L1");
    expect(line2?.className).toContain("bg-diff-add-bg");
    expect(line3?.className).toContain("bg-diff-add-bg");
    // Line 1 is Context — should not have add highlight
    expect(line1?.className).not.toContain("bg-diff-add-bg");
  });

  // --- Inline comments in file view ---

  it("gutter click opens inline comment form in file view", async () => {
    const user = userEvent.setup();
    mockGetFileContent.mockResolvedValueOnce({
      path: "src/main.ts",
      language: "typescript",
      lines: [
        { line_no: 1, content: "line one" },
        { line_no: 2, content: "line two" },
      ],
    });
    await renderDiff();
    await user.click(screen.getByRole("button", { name: "File" }));
    await screen.findByText("line one");

    // In file view, each line has a gutter button
    // There are toggle buttons (Diff, File, New, Old) + 2 gutter buttons = 6
    const buttons = screen.getAllByRole("button");
    const gutterButtons = buttons.filter(
      (b) => b.className.includes("w-6") && b.className.includes("leading-6"),
    );
    expect(gutterButtons.length).toBe(2);
    await user.click(gutterButtons[0]);
    expect(screen.getByPlaceholderText("Add a comment...")).toBeInTheDocument();
  });

  // --- navigateToLine ---

  it("navigateToLine calls scrollIntoView for a line in the diff", async () => {
    const scrollIntoView = vi.fn();
    HTMLElement.prototype.scrollIntoView = scrollIntoView;

    // The navigateToLine effect fires before the diff loads (diffLineNumbers
    // empty), so it first switches to file view. Once the diff loads and
    // diffLineNumbers is populated, the effect re-runs and finds line 2.
    mockGetFileContent.mockResolvedValueOnce({
      path: "src/main.ts",
      language: "typescript",
      lines: [{ line_no: 2, content: "const x = 2;" }],
    });
    mockGetFileDiff.mockResolvedValueOnce(FIXTURE);
    render(DiffView, {
      props: {
        reviewId: "rev-1",
        filePath: "src/main.ts",
        fileStatus: "Modified" as const,
        threads: [],
        navigateToLine: 2,
      },
    });

    await vi.waitFor(() => {
      expect(scrollIntoView).toHaveBeenCalled();
    });
  });

  it("navigateToLine switches to file view for lines outside diff", async () => {
    mockGetFileContent.mockResolvedValueOnce({
      path: "src/main.ts",
      language: "typescript",
      lines: [
        { line_no: 1, content: "line 1" },
        { line_no: 7, content: "line 7" },
      ],
    });
    mockGetFileDiff.mockResolvedValueOnce(FIXTURE);
    // Line 7 is not in the diff (diff has lines 1-4 and 11)
    render(DiffView, {
      props: {
        reviewId: "rev-1",
        filePath: "src/main.ts",
        fileStatus: "Modified" as const,
        threads: [],
        navigateToLine: 7,
      },
    });

    // Should switch to file view and load content
    await screen.findByText("line 7");
    expect(mockGetFileContent).toHaveBeenCalledWith(
      "rev-1",
      "src/main.ts",
      "new",
    );
  });

  // --- onDiffLinesKnown ---

  it("calls onDiffLinesKnown with diff line numbers after load", async () => {
    const onDiffLinesKnown = vi.fn();
    await renderDiff([], { onDiffLinesKnown });
    expect(onDiffLinesKnown).toHaveBeenCalled();
    const lines: Set<number> = onDiffLinesKnown.mock.calls[0][0];
    // FIXTURE has new_line_no: 1, 2, 3, 4, 11
    expect(lines).toEqual(new Set([1, 2, 3, 4, 11]));
  });
});
