<script lang="ts">
  import { listReviews, deleteReview, deleteClosedReviews } from "../lib/api";
  import { navigate } from "../lib/router.svelte";
  import { onEvent, onReconnect } from "../lib/ws";
  import type { ReviewResponse } from "../lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  type StatusFilter = "Open" | "Closed" | "All";
  type SortField =
    | "updated_desc"
    | "updated_asc"
    | "files"
    | "threads"
    | "open_threads";

  let reviews = $state<ReviewResponse[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let statusFilter = $state<StatusFilter>("Open");
  let sortField = $state<SortField>("updated_desc");
  let searchQuery = $state("");

  // Confirmation dialog state
  let confirmDialog = $state<{
    title: string;
    message: string;
    onconfirm: () => void;
  } | null>(null);

  function filteredAndSortedReviews(): ReviewResponse[] {
    let filtered = reviews;

    // Status filter
    if (statusFilter !== "All") {
      filtered = filtered.filter((r) => r.status === statusFilter);
    }

    // Text search
    if (searchQuery.trim()) {
      const q = searchQuery.trim().toLowerCase();
      filtered = filtered.filter(
        (r) => r.title && r.title.toLowerCase().includes(q),
      );
    }

    // Sort
    return [...filtered].sort((a, b) => {
      switch (sortField) {
        case "updated_desc":
          return (
            new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
          );
        case "updated_asc":
          return (
            new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime()
          );
        case "files":
          return b.file_count - a.file_count;
        case "threads":
          return b.thread_count - a.thread_count;
        case "open_threads":
          return b.open_thread_count - a.open_thread_count;
      }
    });
  }

  function statusCount(status: StatusFilter): number {
    if (status === "All") return reviews.length;
    return reviews.filter((r) => r.status === status).length;
  }

  function closedCount(): number {
    return reviews.filter((r) => r.status === "Closed").length;
  }

  function relativeTime(iso: string): string {
    const seconds = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
    if (seconds < 60) return "just now";
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }

  function loadReviews() {
    listReviews()
      .then((data) => {
        reviews = data;
      })
      .catch((e) => {
        error = e.message;
      })
      .finally(() => {
        loading = false;
      });
  }

  function handleDeleteReview(e: MouseEvent, review: ReviewResponse) {
    e.stopPropagation();
    confirmDialog = {
      title: "Delete review",
      message: `Delete "${review.title ?? "Untitled review"}"? This cannot be undone.`,
      onconfirm: async () => {
        confirmDialog = null;
        try {
          await deleteReview(review.id);
          reviews = reviews.filter((r) => r.id !== review.id);
        } catch (err) {
          // Review may have already been deleted
          if (err instanceof Error && err.message.includes("404")) {
            reviews = reviews.filter((r) => r.id !== review.id);
          }
        }
      },
    };
  }

  function handleClearClosed() {
    const count = closedCount();
    confirmDialog = {
      title: "Clear closed reviews",
      message: `Delete ${count} closed review${count !== 1 ? "s" : ""}? This cannot be undone.`,
      onconfirm: async () => {
        confirmDialog = null;
        try {
          await deleteClosedReviews();
          reviews = reviews.filter((r) => r.status !== "Closed");
        } catch {
          // Reload to get fresh state
          loadReviews();
        }
      },
    };
  }

  $effect(() => {
    loadReviews();

    const unsubs = [
      onEvent("review_created", (event) => {
        const newReview = event.payload as ReviewResponse;
        reviews = [...reviews, newReview];
      }),
      onEvent("review_status_changed", (event) => {
        const { status } = event.payload as { status: string };
        reviews = reviews.map((r) =>
          r.id === event.review_id
            ? { ...r, status: status as ReviewResponse["status"] }
            : r,
        );
      }),
      onEvent("review_deleted", (event) => {
        reviews = reviews.filter((r) => r.id !== event.review_id);
      }),
      onReconnect(() => loadReviews()),
    ];

    return () => unsubs.forEach((fn) => fn());
  });
</script>

<div class="min-h-screen">
  <header
    class="max-w-5xl mx-auto px-8 pt-12 pb-4 flex items-center justify-between"
  >
    <h1 class="text-xl font-semibold">preflight</h1>
    {#if closedCount() > 0}
      <button
        class="text-sm text-text-muted hover:text-badge-deleted transition-colors cursor-pointer"
        onclick={handleClearClosed}
      >
        Clear closed ({closedCount()})
      </button>
    {/if}
  </header>

  <main class="max-w-5xl mx-auto px-8">
    {#if loading}
      <p class="text-text-muted">Loading...</p>
    {:else if error}
      <p class="text-badge-deleted">Error: {error}</p>
    {:else if reviews.length === 0}
      <p class="text-text-muted">No reviews yet.</p>
    {:else}
      <!-- Filter/sort toolbar -->
      <div class="flex items-center gap-3 mb-3">
        <!-- Status tabs -->
        <div class="flex items-center gap-1 text-sm">
          {#each ["Open", "Closed", "All"] as status (status)}
            <button
              class="px-2.5 py-1 rounded-md transition-colors cursor-pointer {statusFilter ===
              status
                ? 'bg-bg-hover text-text'
                : 'text-text-muted hover:text-text'}"
              onclick={() => (statusFilter = status as StatusFilter)}
            >
              {status}
              <span class="text-text-faint ml-0.5"
                >{statusCount(status as StatusFilter)}</span
              >
            </button>
          {/each}
        </div>

        <!-- Search -->
        <div class="flex-1">
          <input
            type="text"
            placeholder="Filter by title..."
            bind:value={searchQuery}
            class="w-full bg-transparent text-sm text-text placeholder:text-text-faint border border-border rounded-md px-2.5 py-1 outline-none focus:border-text-muted transition-colors"
          />
        </div>

        <!-- Sort dropdown -->
        <select
          bind:value={sortField}
          class="bg-bg-surface text-sm text-text-muted border border-border rounded-md px-2 py-1 outline-none cursor-pointer focus:border-text-muted transition-colors"
        >
          <option value="updated_desc">Newest</option>
          <option value="updated_asc">Oldest</option>
          <option value="files">Most files</option>
          <option value="threads">Most threads</option>
          <option value="open_threads">Most open threads</option>
        </select>
      </div>

      <!-- Review list -->
      {#if filteredAndSortedReviews().length === 0}
        <p class="text-text-muted py-4">No matching reviews.</p>
      {:else}
        <ul class="space-y-1">
          {#each filteredAndSortedReviews() as review (review.id)}
            <li class="group">
              <div
                class="grid items-center px-4 py-3 rounded-lg hover:bg-bg-hover transition-colors cursor-pointer"
                style="grid-template-columns: 1fr auto;"
                role="button"
                tabindex="0"
                onclick={() => navigate(`/reviews/${review.id}`)}
                onkeydown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    navigate(`/reviews/${review.id}`);
                  }
                }}
              >
                <div class="flex items-center gap-3 min-w-0">
                  <span class="truncate font-medium">
                    {review.title ?? "Untitled review"}
                  </span>
                  <span
                    class="shrink-0 text-xs px-2 py-0.5 rounded-full {review.status ===
                    'Open'
                      ? 'bg-status-open/15 text-status-open'
                      : 'bg-bg-hover text-text-faint'}"
                  >
                    {review.status}
                  </span>
                </div>
                <div
                  class="flex items-center gap-4 text-sm text-text-muted ml-4"
                >
                  <span class="w-14 text-right">{review.file_count} files</span>
                  <span
                    class="w-28 text-right {review.open_thread_count > 0
                      ? 'text-status-open'
                      : ''}"
                    >{review.thread_count} threads{review.open_thread_count > 0
                      ? ` (${review.open_thread_count} open)`
                      : ""}</span
                  >
                  <span class="w-16 text-right text-text-faint"
                    >{relativeTime(review.updated_at)}</span
                  >
                  <button
                    class="opacity-0 group-hover:opacity-100 text-text-faint hover:text-badge-deleted transition-all cursor-pointer p-1 -m-1"
                    onclick={(e) => handleDeleteReview(e, review)}
                    title="Delete review"
                  >
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      width="16"
                      height="16"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      <polyline points="3 6 5 6 21 6"></polyline>
                      <path
                        d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                      ></path>
                    </svg>
                  </button>
                </div>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </main>
</div>

{#if confirmDialog}
  <ConfirmDialog
    title={confirmDialog.title}
    message={confirmDialog.message}
    onconfirm={confirmDialog.onconfirm}
    oncancel={() => (confirmDialog = null)}
  />
{/if}
