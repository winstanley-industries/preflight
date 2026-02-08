<script lang="ts">
  import { listReviews } from "../lib/api";
  import { navigate } from "../lib/router.svelte";
  import { onEvent, onReconnect } from "../lib/ws";
  import type { ReviewResponse } from "../lib/types";

  let reviews = $state<ReviewResponse[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);

  function sortedReviews(): ReviewResponse[] {
    return [...reviews].sort((a, b) => {
      if (a.status !== b.status) return a.status === "Open" ? -1 : 1;
      return (
        new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
      );
    });
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
      onReconnect(() => loadReviews()),
    ];

    return () => unsubs.forEach((fn) => fn());
  });
</script>

<div class="min-h-screen">
  <header class="max-w-2xl mx-auto px-6 pt-12 pb-6">
    <h1 class="text-xl font-semibold">preflight</h1>
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
          <li>
            <button
              class="w-full text-left px-4 py-3 rounded-lg hover:bg-bg-hover transition-colors cursor-pointer"
              onclick={() => navigate(`/reviews/${review.id}`)}
            >
              <div class="flex items-center justify-between gap-4">
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
                </div>
              </div>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </main>
</div>
