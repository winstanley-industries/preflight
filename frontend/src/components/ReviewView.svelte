<script lang="ts">
  import {
    getReview,
    listFiles,
    listRevisions,
    listThreads,
    createRevision,
    ApiError,
  } from "../lib/api";
  import { navigate } from "../lib/router.svelte";
  import { onEvent, onReconnect } from "../lib/ws";
  import type {
    FileListEntry,
    ReviewResponse,
    RevisionResponse,
    ThreadResponse,
  } from "../lib/types";
  import FileTree from "./FileTree.svelte";
  import DiffView from "./DiffView.svelte";
  import ThreadPanel from "./ThreadPanel.svelte";
  import RevisionTimeline from "./RevisionTimeline.svelte";

  interface Props {
    reviewId: string;
  }

  let { reviewId }: Props = $props();

  let review = $state<ReviewResponse | null>(null);
  let files = $state<FileListEntry[]>([]);
  let threads = $state<ThreadResponse[]>([]);
  let revisions = $state<RevisionResponse[]>([]);
  let selectedRevision = $state<number>(0);
  let selectedFile = $state<string | null>(null);
  let error = $state<string | null>(null);
  let refreshMessage = $state<string | null>(null);
  let threadsPanelOpen = $state(true);
  let highlightThreadId = $state<string | null>(null);
  let navigateToLine = $state<number | null>(null);
  let diffLines = $state<Set<number>>(new Set());

  let selectedFileStatus = $derived(
    files.find((f) => f.path === selectedFile)?.status ?? "Modified",
  );

  // Interdiff: compare two revisions via shift+click on the timeline
  let compareFrom = $state<number | null>(null);
  let interdiffParams = $derived(
    compareFrom != null ? { from: compareFrom, to: selectedRevision } : null,
  );

  async function load() {
    try {
      const [r, revs] = await Promise.all([
        getReview(reviewId),
        listRevisions(reviewId),
      ]);
      review = r;
      revisions = revs;
      const latest =
        revs.length > 0
          ? Math.max(...revs.map((rev) => rev.revision_number))
          : 0;
      selectedRevision = latest;
      const f = await listFiles(reviewId, latest || undefined);
      files = f;
      if (f.length > 0 && !selectedFile) {
        selectedFile = f[0].path;
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : "Failed to load review";
    }
  }

  async function selectRevision(revisionNumber: number) {
    compareFrom = null;
    try {
      const newFiles = await listFiles(reviewId, revisionNumber);
      const currentFileExists =
        selectedFile && newFiles.find((f) => f.path === selectedFile);
      if (!currentFileExists) {
        selectedFile = newFiles.length > 0 ? newFiles[0].path : null;
      }
      files = newFiles;
      selectedRevision = revisionNumber;
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : "Failed to load files";
    }
  }

  async function handleCompare(from: number, to: number) {
    compareFrom = from;
    // Load file lists from both revisions and merge to show the union
    try {
      const [fromFiles, toFiles] = await Promise.all([
        listFiles(reviewId, from),
        listFiles(reviewId, to),
      ]);
      const merged: FileListEntry[] = [...toFiles];
      for (const f of fromFiles) {
        if (!merged.some((m) => m.path === f.path)) {
          merged.push(f);
        }
      }
      const currentFileExists =
        selectedFile && merged.find((f) => f.path === selectedFile);
      if (!currentFileExists) {
        selectedFile = merged.length > 0 ? merged[0].path : null;
      }
      files = merged;
      selectedRevision = to;
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : "Failed to load files";
    }
  }

  async function handleRefresh() {
    refreshMessage = null;
    try {
      await createRevision(reviewId, { trigger: "Manual" });
      const revs = await listRevisions(reviewId);
      revisions = revs;
      const latest = Math.max(...revs.map((rev) => rev.revision_number));
      await selectRevision(latest);
    } catch (e: unknown) {
      if (e instanceof ApiError && e.status === 400) {
        refreshMessage = "No changes detected";
        setTimeout(() => {
          refreshMessage = null;
        }, 3000);
      } else {
        error = e instanceof Error ? e.message : "Failed to create revision";
      }
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

  $effect(() => {
    const unsubs = [
      onEvent("review_status_changed", (event) => {
        if (event.review_id !== reviewId || !review) return;
        const { status } = event.payload as {
          status: ReviewResponse["status"];
        };
        review = { ...review, status };
      }),
      onEvent("revision_created", (event) => {
        if (event.review_id !== reviewId) return;
        // Refetch revisions and file list
        listRevisions(reviewId).then((revs) => {
          revisions = revs;
          const latest = Math.max(...revs.map((r) => r.revision_number));
          selectRevision(latest);
        });
      }),
      onEvent("thread_created", (event) => {
        if (event.review_id !== reviewId) return;
        if (selectedFile) loadThreads(selectedFile);
      }),
      onEvent("comment_added", (event) => {
        if (event.review_id !== reviewId) return;
        if (selectedFile) loadThreads(selectedFile);
      }),
      onEvent("thread_status_changed", (event) => {
        if (event.review_id !== reviewId) return;
        if (selectedFile) loadThreads(selectedFile);
      }),
      onReconnect(() => load()),
    ];

    return () => unsubs.forEach((fn) => fn());
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
    <header
      class="flex items-center gap-3 px-4 py-2 border-b border-border shrink-0"
    >
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

    <!-- Revision timeline -->
    {#if revisions.length > 0}
      <div class="border-b border-border shrink-0">
        <RevisionTimeline
          {revisions}
          {selectedRevision}
          {compareFrom}
          onSelect={selectRevision}
          onCompare={handleCompare}
          onClearCompare={() => (compareFrom = null)}
          onRefresh={handleRefresh}
        />
      </div>
      {#if refreshMessage}
        <div
          class="px-4 py-1 text-xs text-text-faint bg-bg-surface border-b border-border shrink-0"
        >
          {refreshMessage}
        </div>
      {/if}
    {/if}

    <!-- Three-panel body -->
    <div class="flex flex-1 min-h-0">
      <!-- File tree -->
      <aside class="w-60 border-r border-border overflow-y-auto shrink-0">
        <FileTree
          {files}
          {selectedFile}
          onSelect={(path) => {
            selectedFile = path;
            navigateToLine = null;
            diffLines = new Set();
          }}
        />
      </aside>

      <!-- Diff -->
      <main class="flex-1 overflow-y-auto min-w-0">
        {#if selectedFile}
          <DiffView
            {reviewId}
            filePath={selectedFile}
            {threads}
            fileStatus={selectedFileStatus}
            revision={selectedRevision || undefined}
            interdiff={interdiffParams}
            {navigateToLine}
            onDiffLinesKnown={(lines) => {
              diffLines = lines;
            }}
            onThreadCreated={(threadId) => {
              highlightThreadId = threadId;
              if (selectedFile) loadThreads(selectedFile);
              setTimeout(() => {
                highlightThreadId = null;
              }, 1000);
            }}
          />
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
            {threads}
            {highlightThreadId}
            {diffLines}
            onNavigateToThread={(line) => {
              navigateToLine = line;
              setTimeout(() => {
                navigateToLine = null;
              }, 100);
            }}
            onThreadsChanged={() => {
              if (selectedFile) loadThreads(selectedFile);
            }}
          />
        </aside>
      {/if}
    </div>
  </div>
{/if}
