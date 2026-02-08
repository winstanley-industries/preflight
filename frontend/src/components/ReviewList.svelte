<script lang="ts">
  import { listReviews, deleteReview, deleteClosedReviews } from "../lib/api";
  import { navigate } from "../lib/router.svelte";
  import { onEvent, onReconnect } from "../lib/ws";
  import type { ReviewResponse } from "../lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  let reviews = $state<ReviewResponse[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);

  // Confirmation dialog state
  let confirmDialog = $state<{
    title: string;
    message: string;
    onconfirm: () => void;
  } | null>(null);

  function sortedReviews(): ReviewResponse[] {
    return [...reviews].sort((a, b) => {
      if (a.status !== b.status) return a.status === "Open" ? -1 : 1;
      return (
        new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
      );
    });
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
    class="max-w-2xl mx-auto px-6 pt-12 pb-6 flex items-center justify-between"
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

  <main class="max-w-2xl mx-auto px-6">
    {#if loading}
      <p class="text-text-muted">Loading...</p>
    {:else if error}
      <p class="text-badge-deleted">Error: {error}</p>
    {:else if reviews.length === 0}
      <p class="text-text-muted">No reviews yet.</p>
    {:else}
      <ul class="space-y-1">
        {#each sortedReviews() as review (review.id)}
          <li class="group">
            <div
              class="flex items-center px-4 py-3 rounded-lg hover:bg-bg-hover transition-colors cursor-pointer"
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
              <div class="flex items-center justify-between gap-4 w-full">
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
                  class="flex items-center gap-4 shrink-0 text-sm text-text-muted"
                >
                  <span>{review.file_count} files</span>
                  <span
                    class={review.open_thread_count > 0
                      ? "text-status-open"
                      : ""}
                    >{review.thread_count} threads{review.open_thread_count > 0
                      ? ` (${review.open_thread_count} open)`
                      : ""}</span
                  >
                  <span class="text-text-faint"
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
            </div>
          </li>
        {/each}
      </ul>
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
