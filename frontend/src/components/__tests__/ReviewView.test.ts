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
  getAgentPresence: vi.fn(() => Promise.resolve({ connected: false })),
  requestRevision: vi.fn(() => Promise.resolve()),
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

import {
  updateReviewStatus,
  getAgentPresence,
  requestRevision,
} from "../../lib/api";
const mockUpdateReviewStatus = vi.mocked(updateReviewStatus);
const mockGetAgentPresence = vi.mocked(getAgentPresence);
const mockRequestRevision = vi.mocked(requestRevision);

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

describe("ReviewView agent presence and revision", () => {
  beforeEach(() => {
    cleanup();
    resolvedReview = { ...mockReview, status: "Open", open_thread_count: 2 };
    mockGetAgentPresence.mockResolvedValue({ connected: false });
    mockRequestRevision.mockClear();
    mockRequestRevision.mockResolvedValue(undefined);
  });

  it("shows 'No agent' when agent is not connected", async () => {
    await renderAndWait();
    expect(screen.getByText("No agent")).toBeInTheDocument();
  });

  it("shows 'Agent connected' when agent is connected", async () => {
    mockGetAgentPresence.mockResolvedValue({ connected: true });
    await renderAndWait();
    expect(screen.getByText("Agent connected")).toBeInTheDocument();
  });

  it("shows 'Ready for revision' button when review is open with threads", async () => {
    mockGetAgentPresence.mockResolvedValue({ connected: true });
    await renderAndWait();
    expect(
      screen.getByRole("button", { name: "Ready for revision" }),
    ).toBeInTheDocument();
  });

  it("hides 'Ready for revision' button when no open threads", async () => {
    resolvedReview = { ...mockReview, status: "Open", open_thread_count: 0 };
    await renderAndWait();
    expect(
      screen.queryByRole("button", { name: "Ready for revision" }),
    ).not.toBeInTheDocument();
  });

  it("disables 'Ready for revision' button when agent is not connected", async () => {
    mockGetAgentPresence.mockResolvedValue({ connected: false });
    await renderAndWait();
    const btn = screen.getByRole("button", { name: "Ready for revision" });
    expect(btn).toBeDisabled();
  });

  it("calls requestRevision on click", async () => {
    const user = userEvent.setup();
    mockGetAgentPresence.mockResolvedValue({ connected: true });
    await renderAndWait();

    await user.click(
      screen.getByRole("button", { name: "Ready for revision" }),
    );

    expect(mockRequestRevision).toHaveBeenCalledWith(REVIEW_ID);
  });

  it("shows 'Revision requested...' after clicking", async () => {
    const user = userEvent.setup();
    mockGetAgentPresence.mockResolvedValue({ connected: true });
    await renderAndWait();

    await user.click(
      screen.getByRole("button", { name: "Ready for revision" }),
    );

    await vi.waitFor(() => {
      expect(screen.getByText("Revision requested...")).toBeInTheDocument();
    });
  });
});
