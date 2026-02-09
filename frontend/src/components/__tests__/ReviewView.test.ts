import { render, screen, cleanup } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import type {
  ReviewResponse,
  FileListEntry,
  RevisionResponse,
} from "../../lib/types";
import ReviewView from "../ReviewView.svelte";

const REVIEW_ID = "test-review-id";

const mockReview: ReviewResponse = {
  id: REVIEW_ID,
  title: "Test review",
  status: "Open",
  file_count: 1,
  thread_count: 0,
  open_thread_count: 0,
  revision_count: 1,
  created_at: "2025-01-01T00:00:00Z",
  updated_at: "2025-01-01T00:00:00Z",
};

const mockFiles: FileListEntry[] = [
  {
    path: "src/main.ts",
    status: "Modified",
    thread_count: 0,
    open_thread_count: 0,
  },
];

const mockRevisions: RevisionResponse[] = [
  {
    id: "rev-1",
    review_id: REVIEW_ID,
    revision_number: 1,
    trigger: "Manual",
    message: null,
    created_at: "2025-01-01T00:00:00Z",
    file_count: 1,
  },
];

let resolvedReview = { ...mockReview };

vi.mock("../../lib/api", () => ({
  getReview: vi.fn(() => Promise.resolve(resolvedReview)),
  listFiles: vi.fn(() => Promise.resolve(mockFiles)),
  listRevisions: vi.fn(() => Promise.resolve(mockRevisions)),
  listThreads: vi.fn(() => Promise.resolve([])),
  createRevision: vi.fn(),
  updateReviewStatus: vi.fn(() => Promise.resolve()),
  ApiError: class ApiError extends Error {
    status: number;
    constructor(status: number, message: string) {
      super(message);
      this.status = status;
    }
  },
}));

vi.mock("../../lib/router.svelte", () => ({
  navigate: vi.fn(),
}));

// ReviewView uses ResizeObserver for pane clamping
vi.stubGlobal(
  "ResizeObserver",
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  },
);

// ReviewView reads localStorage for pane widths
vi.stubGlobal(
  "localStorage",
  Object.defineProperties(
    {},
    {
      getItem: { value: vi.fn(() => null), writable: true },
      setItem: { value: vi.fn(), writable: true },
      removeItem: { value: vi.fn(), writable: true },
      clear: { value: vi.fn(), writable: true },
    },
  ),
);

vi.mock("../../lib/ws", () => ({
  onEvent: vi.fn(() => () => {}),
  onReconnect: vi.fn(() => () => {}),
}));

import { updateReviewStatus } from "../../lib/api";
const mockUpdateReviewStatus = vi.mocked(updateReviewStatus);

async function renderAndWait() {
  render(ReviewView, { props: { reviewId: REVIEW_ID } });
  await vi.waitFor(() => {
    expect(screen.queryByText("Loading...")).not.toBeInTheDocument();
  });
}

describe("ReviewView status toggle", () => {
  beforeEach(() => {
    cleanup();
    resolvedReview = { ...mockReview, status: "Open" };
    mockUpdateReviewStatus.mockClear();
    mockUpdateReviewStatus.mockResolvedValue(undefined);
  });

  it("shows 'Close review' button when review is open", async () => {
    await renderAndWait();

    expect(
      screen.getByRole("button", { name: "Close review" }),
    ).toBeInTheDocument();
  });

  it("shows 'Reopen review' button when review is closed", async () => {
    resolvedReview = { ...mockReview, status: "Closed" };
    await renderAndWait();

    expect(
      screen.getByRole("button", { name: "Reopen review" }),
    ).toBeInTheDocument();
  });

  it("calls updateReviewStatus with Closed when closing", async () => {
    const user = userEvent.setup();
    await renderAndWait();

    await user.click(screen.getByRole("button", { name: "Close review" }));

    expect(mockUpdateReviewStatus).toHaveBeenCalledWith(REVIEW_ID, {
      status: "Closed",
    });
  });

  it("calls updateReviewStatus with Open when reopening", async () => {
    const user = userEvent.setup();
    resolvedReview = { ...mockReview, status: "Closed" };
    await renderAndWait();

    await user.click(screen.getByRole("button", { name: "Reopen review" }));

    expect(mockUpdateReviewStatus).toHaveBeenCalledWith(REVIEW_ID, {
      status: "Open",
    });
  });

  it("toggles button text after closing", async () => {
    const user = userEvent.setup();
    await renderAndWait();

    await user.click(screen.getByRole("button", { name: "Close review" }));

    await vi.waitFor(() => {
      expect(
        screen.getByRole("button", { name: "Reopen review" }),
      ).toBeInTheDocument();
    });
  });

  it("toggles button text after reopening", async () => {
    const user = userEvent.setup();
    resolvedReview = { ...mockReview, status: "Closed" };
    await renderAndWait();

    await user.click(screen.getByRole("button", { name: "Reopen review" }));

    await vi.waitFor(() => {
      expect(
        screen.getByRole("button", { name: "Close review" }),
      ).toBeInTheDocument();
    });
  });

  it("updates the status badge after closing", async () => {
    const user = userEvent.setup();
    await renderAndWait();

    expect(screen.getByText("Open")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Close review" }));

    await vi.waitFor(() => {
      expect(screen.getByText("Closed")).toBeInTheDocument();
    });
  });
});
