import { render, screen, cleanup, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ReviewResponse } from "../../lib/types";
import ReviewList from "../ReviewList.svelte";

// Mock API
let mockReviews: ReviewResponse[] = [];

vi.mock("../../lib/api", () => ({
  listReviews: vi.fn(() => Promise.resolve(mockReviews)),
  deleteReview: vi.fn(() => Promise.resolve()),
  deleteClosedReviews: vi.fn(() => Promise.resolve()),
}));

vi.mock("../../lib/router.svelte", () => ({
  navigate: vi.fn(),
}));

vi.mock("../../lib/ws", () => ({
  onEvent: vi.fn(() => () => {}),
  onReconnect: vi.fn(() => () => {}),
}));

function makeReview(overrides: Partial<ReviewResponse> = {}): ReviewResponse {
  return {
    id: crypto.randomUUID(),
    title: "Test review",
    status: "Open",
    file_count: 5,
    thread_count: 2,
    open_thread_count: 1,
    revision_count: 1,
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
    ...overrides,
  };
}

async function renderAndWait() {
  render(ReviewList);
  // Wait for the async listReviews() to resolve and render
  await vi.waitFor(() => {
    expect(screen.queryByText("Loading...")).not.toBeInTheDocument();
  });
}

describe("ReviewList filtering and sorting", () => {
  beforeEach(() => {
    cleanup();
    mockReviews = [];
  });

  describe("status filter", () => {
    it("defaults to showing only open reviews", async () => {
      mockReviews = [
        makeReview({ title: "Open one", status: "Open" }),
        makeReview({ title: "Closed one", status: "Closed" }),
      ];
      await renderAndWait();

      expect(screen.getByText("Open one")).toBeInTheDocument();
      expect(screen.queryByText("Closed one")).not.toBeInTheDocument();
    });

    it("shows closed reviews when Closed tab is clicked", async () => {
      const user = userEvent.setup();
      mockReviews = [
        makeReview({ title: "Open one", status: "Open" }),
        makeReview({ title: "Closed one", status: "Closed" }),
      ];
      await renderAndWait();

      await user.click(screen.getByRole("button", { name: /Closed/ }));

      expect(screen.queryByText("Open one")).not.toBeInTheDocument();
      expect(screen.getByText("Closed one")).toBeInTheDocument();
    });

    it("shows all reviews when All tab is clicked", async () => {
      const user = userEvent.setup();
      mockReviews = [
        makeReview({ title: "Open one", status: "Open" }),
        makeReview({ title: "Closed one", status: "Closed" }),
      ];
      await renderAndWait();

      await user.click(screen.getByRole("button", { name: /All/ }));

      expect(screen.getByText("Open one")).toBeInTheDocument();
      expect(screen.getByText("Closed one")).toBeInTheDocument();
    });

    it("displays correct counts on status tabs", async () => {
      mockReviews = [
        makeReview({ status: "Open" }),
        makeReview({ status: "Open" }),
        makeReview({ status: "Closed" }),
      ];
      await renderAndWait();

      // The filter toolbar is in a div before the review list
      const toolbar = screen
        .getByPlaceholderText("Filter by title...")
        .closest("div.flex") as HTMLElement;
      const buttons = within(toolbar).getAllByRole("button");
      // Buttons are: Open, Closed, All
      const [openBtn, closedBtn, allBtn] = buttons;

      expect(within(openBtn).getByText("2")).toBeInTheDocument();
      expect(within(closedBtn).getByText("1")).toBeInTheDocument();
      expect(within(allBtn).getByText("3")).toBeInTheDocument();
    });
  });

  describe("text search", () => {
    it("filters reviews by title", async () => {
      const user = userEvent.setup();
      mockReviews = [
        makeReview({ title: "Auth refactor", status: "Open" }),
        makeReview({ title: "Bug fix login", status: "Open" }),
      ];
      await renderAndWait();

      const searchInput = screen.getByPlaceholderText("Filter by title...");
      await user.type(searchInput, "auth");

      expect(screen.getByText("Auth refactor")).toBeInTheDocument();
      expect(screen.queryByText("Bug fix login")).not.toBeInTheDocument();
    });

    it("search is case-insensitive", async () => {
      const user = userEvent.setup();
      mockReviews = [makeReview({ title: "Auth Refactor", status: "Open" })];
      await renderAndWait();

      const searchInput = screen.getByPlaceholderText("Filter by title...");
      await user.type(searchInput, "AUTH REFACTOR");

      expect(screen.getByText("Auth Refactor")).toBeInTheDocument();
    });

    it("shows no matching reviews message when search has no results", async () => {
      const user = userEvent.setup();
      mockReviews = [makeReview({ title: "Auth refactor", status: "Open" })];
      await renderAndWait();

      const searchInput = screen.getByPlaceholderText("Filter by title...");
      await user.type(searchInput, "zzzzz");

      expect(screen.getByText("No matching reviews.")).toBeInTheDocument();
    });
  });

  describe("sorting", () => {
    it("defaults to newest first", async () => {
      mockReviews = [
        makeReview({
          title: "Older",
          status: "Open",
          updated_at: "2025-01-01T00:00:00Z",
        }),
        makeReview({
          title: "Newer",
          status: "Open",
          updated_at: "2025-06-01T00:00:00Z",
        }),
      ];
      await renderAndWait();

      const listItems = screen.getByRole("list").querySelectorAll("li");
      const titles = Array.from(listItems).map((li) =>
        li.querySelector(".font-medium")?.textContent?.trim(),
      );
      expect(titles).toEqual(["Newer", "Older"]);
    });

    it("sorts oldest first when selected", async () => {
      const user = userEvent.setup();
      mockReviews = [
        makeReview({
          title: "Older",
          status: "Open",
          updated_at: "2025-01-01T00:00:00Z",
        }),
        makeReview({
          title: "Newer",
          status: "Open",
          updated_at: "2025-06-01T00:00:00Z",
        }),
      ];
      await renderAndWait();

      await user.selectOptions(screen.getByRole("combobox"), "updated_asc");

      const listItems = screen.getByRole("list").querySelectorAll("li");
      const titles = Array.from(listItems).map((li) =>
        li.querySelector(".font-medium")?.textContent?.trim(),
      );
      expect(titles).toEqual(["Older", "Newer"]);
    });

    it("sorts by most files when selected", async () => {
      const user = userEvent.setup();
      mockReviews = [
        makeReview({
          title: "Few files",
          status: "Open",
          file_count: 2,
        }),
        makeReview({
          title: "Many files",
          status: "Open",
          file_count: 20,
        }),
      ];
      await renderAndWait();

      await user.selectOptions(screen.getByRole("combobox"), "files");

      const listItems = screen.getByRole("list").querySelectorAll("li");
      const titles = Array.from(listItems).map((li) =>
        li.querySelector(".font-medium")?.textContent?.trim(),
      );
      expect(titles).toEqual(["Many files", "Few files"]);
    });

    it("sorts by most open threads when selected", async () => {
      const user = userEvent.setup();
      mockReviews = [
        makeReview({
          title: "Few open",
          status: "Open",
          open_thread_count: 1,
        }),
        makeReview({
          title: "Many open",
          status: "Open",
          open_thread_count: 10,
        }),
      ];
      await renderAndWait();

      await user.selectOptions(screen.getByRole("combobox"), "open_threads");

      const listItems = screen.getByRole("list").querySelectorAll("li");
      const titles = Array.from(listItems).map((li) =>
        li.querySelector(".font-medium")?.textContent?.trim(),
      );
      expect(titles).toEqual(["Many open", "Few open"]);
    });
  });

  describe("combined filters", () => {
    it("search and status filter work together", async () => {
      const user = userEvent.setup();
      mockReviews = [
        makeReview({ title: "Auth refactor", status: "Open" }),
        makeReview({ title: "Auth cleanup", status: "Closed" }),
        makeReview({ title: "Bug fix", status: "Open" }),
      ];
      await renderAndWait();

      // Default is Open, so "Auth cleanup" (Closed) is hidden
      // Now search for "auth"
      const searchInput = screen.getByPlaceholderText("Filter by title...");
      await user.type(searchInput, "auth");

      expect(screen.getByText("Auth refactor")).toBeInTheDocument();
      expect(screen.queryByText("Auth cleanup")).not.toBeInTheDocument();
      expect(screen.queryByText("Bug fix")).not.toBeInTheDocument();
    });
  });
});
