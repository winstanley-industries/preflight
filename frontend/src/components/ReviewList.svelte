<script lang="ts">
  import { listReviews } from "../lib/api";
  import { navigate } from "../lib/router.svelte";
  import type { ReviewResponse } from "../lib/types";

  let reviews = $state<ReviewResponse[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);

  function sortedReviews(): ReviewResponse[] {
    return [...reviews].sort((a, b) => {
      if (a.status !== b.status) return a.status === "Open" ? -1 : 1;
      return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
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

  $effect(() => {
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
                    class="shrink-0 text-xs px-2 py-0.5 rounded-full {review.status === 'Open'
                      ? 'bg-status-open/15 text-status-open'
                      : 'bg-bg-hover text-text-faint'}"
                  >
                    {review.status}
                  </span>
                </div>
                <div class="flex items-center gap-4 shrink-0 text-sm text-text-muted">
                  <span>{review.file_count} files</span>
                  <span>{review.thread_count} threads</span>
                  <span class="text-text-faint">{relativeTime(review.updated_at)}</span>
                </div>
              </div>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </main>
</div>
