import { render, screen, cleanup } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import InlineCommentForm from "../InlineCommentForm.svelte";

vi.mock("../../lib/api", () => ({
  createThread: vi.fn(),
}));

import { createThread } from "../../lib/api";
const mockCreateThread = vi.mocked(createThread);

function renderForm(overrides: Record<string, unknown> = {}) {
  return render(InlineCommentForm, {
    props: {
      reviewId: "rev-1",
      filePath: "src/main.ts",
      lineStart: 5,
      lineEnd: 5,
      onSubmit: vi.fn(),
      onCancel: vi.fn(),
      ...overrides,
    },
  });
}

describe("InlineCommentForm", () => {
  beforeEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('renders origin toggle with "Comment" selected by default', () => {
    renderForm();
    const commentBtn = screen.getByRole("button", { name: "Comment" });
    expect(commentBtn.className).toContain("text-accent");
  });

  it("renders correct placeholder for Comment origin", () => {
    renderForm();
    expect(screen.getByPlaceholderText("Add a comment...")).toBeInTheDocument();
  });

  it("renders correct placeholder for ExplanationRequest origin", async () => {
    const user = userEvent.setup();
    renderForm();
    await user.click(
      screen.getByRole("button", { name: "Request Explanation" }),
    );
    expect(
      screen.getByPlaceholderText("What should be explained?"),
    ).toBeInTheDocument();
  });

  it('renders "Line 5" for single line', () => {
    renderForm({ lineStart: 5, lineEnd: 5 });
    expect(screen.getByText("Line 5")).toBeInTheDocument();
  });

  it('"Lines 5–8" for range', () => {
    renderForm({ lineStart: 5, lineEnd: 8 });
    expect(screen.getByText("Lines 5\u20138")).toBeInTheDocument();
  });

  it("calls createThread with correct payload and fires onSubmit", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    mockCreateThread.mockResolvedValueOnce({
      id: "thread-42",
      review_id: "rev-1",
      file_path: "src/main.ts",
      line_start: 5,
      line_end: 5,
      origin: "Comment",
      status: "Open",
      comments: [],
      created_at: "",
      updated_at: "",
    });

    renderForm({ onSubmit });
    await user.type(screen.getByRole("textbox"), "Fix this bug");
    await user.click(screen.getByRole("button", { name: "Submit" }));

    expect(mockCreateThread).toHaveBeenCalledWith("rev-1", {
      file_path: "src/main.ts",
      line_start: 5,
      line_end: 5,
      origin: "Comment",
      body: "Fix this bug",
      author_type: "Human",
    });
    expect(onSubmit).toHaveBeenCalledWith("thread-42");
  });

  it("submit is disabled when body is empty", () => {
    renderForm();
    const submitBtn = screen.getByRole("button", { name: "Submit" });
    expect(submitBtn).toBeDisabled();
  });

  it("cancel button fires onCancel", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    renderForm({ onCancel });
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalled();
  });

  it("Escape key fires onCancel", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    renderForm({ onCancel });
    await user.type(screen.getByRole("textbox"), "test");
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalled();
  });

  it("Cmd+Enter submits", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    mockCreateThread.mockResolvedValueOnce({
      id: "thread-99",
      review_id: "rev-1",
      file_path: "src/main.ts",
      line_start: 5,
      line_end: 5,
      origin: "Comment",
      status: "Open",
      comments: [],
      created_at: "",
      updated_at: "",
    });
    renderForm({ onSubmit });
    await user.type(screen.getByRole("textbox"), "quick fix");
    await user.keyboard("{Meta>}{Enter}{/Meta}");
    expect(mockCreateThread).toHaveBeenCalled();
  });

  it("shows error message when API call fails", async () => {
    const user = userEvent.setup();
    mockCreateThread.mockRejectedValueOnce(new Error("Network error"));
    renderForm();
    await user.type(screen.getByRole("textbox"), "something");
    await user.click(screen.getByRole("button", { name: "Submit" }));
    expect(await screen.findByText("Network error")).toBeInTheDocument();
  });
});
