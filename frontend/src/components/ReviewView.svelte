<script lang="ts">
  import { getReview, listFiles, listThreads } from "../lib/api";
  import { navigate } from "../lib/router.svelte";
  import type { FileListEntry, ReviewResponse, ThreadResponse } from "../lib/types";
  import FileTree from "./FileTree.svelte";
  import DiffView from "./DiffView.svelte";
  import ThreadPanel from "./ThreadPanel.svelte";

  interface Props {
    reviewId: string;
  }

  let { reviewId }: Props = $props();

  let review = $state<ReviewResponse | null>(null);
  let files = $state<FileListEntry[]>([]);
  let threads = $state<ThreadResponse[]>([]);
  let selectedFile = $state<string | null>(null);
  let error = $state<string | null>(null);
  let threadsPanelOpen = $state(true);

  async function load() {
    try {
      const [r, f] = await Promise.all([getReview(reviewId), listFiles(reviewId)]);
      review = r;
      files = f;
      if (f.length > 0 && !selectedFile) {
        selectedFile = f[0].path;
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : "Failed to load review";
    }
  }

  async function loadThreads(filePath: string) {
    try {
      threads = await listThreads(reviewId, filePath);
    } catch {
      threads = [];
    }
  }

  $effect(() => {
    load();
  });

  $effect(() => {
    if (selectedFile) {
      loadThreads(selectedFile);
    }
  });
</script>

{#if error}
  <div class="flex items-center justify-center min-h-screen">
    <p class="text-badge-deleted">{error}</p>
  </div>
{:else if !review}
  <div class="flex items-center justify-center min-h-screen">
    <p class="text-text-muted">Loading...</p>
  </div>
{:else}
  <div class="flex flex-col h-screen">
    <!-- Header -->
    <header class="flex items-center gap-3 px-4 py-2 border-b border-border shrink-0">
      <button
        class="text-text-muted hover:text-text transition-colors cursor-pointer"
        onclick={() => navigate("/")}
      >
        &larr;
      </button>
      <h1 class="text-sm font-medium truncate">
        {review.title ?? "Untitled review"}
      </h1>
      <span
        class="text-xs px-2 py-0.5 rounded-full {review.status === 'Open'
          ? 'bg-status-open/15 text-status-open'
          : 'bg-bg-hover text-text-faint'}"
      >
        {review.status}
      </span>
    </header>

    <!-- Three-panel body -->
    <div class="flex flex-1 min-h-0">
      <!-- File tree -->
      <aside class="w-60 border-r border-border overflow-y-auto shrink-0">
        <FileTree
          {files}
          {selectedFile}
          onSelect={(path) => { selectedFile = path; }}
        />
      </aside>

      <!-- Diff -->
      <main class="flex-1 overflow-y-auto min-w-0">
        {#if selectedFile}
          <DiffView {reviewId} filePath={selectedFile} {threads} />
        {:else}
          <div class="flex items-center justify-center h-full">
            <p class="text-text-muted">Select a file</p>
          </div>
        {/if}
      </main>

      <!-- Thread panel -->
      {#if threadsPanelOpen}
        <aside class="w-80 border-l border-border overflow-y-auto shrink-0">
          <ThreadPanel
            {reviewId}
            {threads}
            onThreadsChanged={() => { if (selectedFile) loadThreads(selectedFile); }}
          />
        </aside>
      {/if}
    </div>
  </div>
{/if}
