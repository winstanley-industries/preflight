<script lang="ts">
  import {
    getReview,
    listFiles,
    listRevisions,
    listThreads,
    createRevision,
    updateReviewStatus,
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
  import ResizeHandle from "./ResizeHandle.svelte";
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
  let statusUpdating = $state(false);
  let threadsPanelOpen = $state(true);
  let highlightThreadId = $state<string | null>(null);
  let navigateToLine = $state<number | null>(null);
  let diffLines = $state<Set<number>>(new Set());
  let mainEl = $state<HTMLElement | null>(null);
  let visibleLine = $state<number | null>(null);
  let suppressSync = false;

  // Resizable pane state
  const FILE_TREE_DEFAULT = 240;
  const FILE_TREE_MIN = 160;
  const FILE_TREE_MAX = 480;
  const THREAD_PANEL_DEFAULT = 320;
  const THREAD_PANEL_MIN = 240;
  const THREAD_PANEL_MAX = 600;
  const DIFF_MIN = 300;

  let fileTreeWidth = $state(
    Number(localStorage.getItem("preflight:fileTreeWidth")) ||
      FILE_TREE_DEFAULT,
  );
  let threadPanelWidth = $state(
    Number(localStorage.getItem("preflight:threadPanelWidth")) ||
      THREAD_PANEL_DEFAULT,
  );
  let isDragging = $state(false);
  let containerEl = $state<HTMLDivElement | null>(null);

  function clampWidths() {
    if (!containerEl) return;
    const totalWidth = containerEl.clientWidth;
    const threadW = threadsPanelOpen ? threadPanelWidth : 0;
    const available = totalWidth - DIFF_MIN;

    if (fileTreeWidth + threadW > available) {
      // Shrink both proportionally to fit
      const ratio = available / (fileTreeWidth + threadW);
      fileTreeWidth = Math.max(
        FILE_TREE_MIN,
        Math.round(fileTreeWidth * ratio),
      );
      if (threadsPanelOpen) {
        threadPanelWidth = Math.max(
          THREAD_PANEL_MIN,
          Math.round(threadPanelWidth * ratio),
        );
      }
    }

    fileTreeWidth = Math.max(
      FILE_TREE_MIN,
      Math.min(FILE_TREE_MAX, fileTreeWidth),
    );
    if (threadsPanelOpen) {
      threadPanelWidth = Math.max(
        THREAD_PANEL_MIN,
        Math.min(THREAD_PANEL_MAX, threadPanelWidth),
      );
    }
  }

  function maxForFileTree(): number {
    if (!containerEl) return FILE_TREE_MAX;
    const threadW = threadsPanelOpen ? threadPanelWidth : 0;
    return Math.min(
      FILE_TREE_MAX,
      containerEl.clientWidth - DIFF_MIN - threadW,
    );
  }

  function maxForThreadPanel(): number {
    if (!containerEl) return THREAD_PANEL_MAX;
    return Math.min(
      THREAD_PANEL_MAX,
      containerEl.clientWidth - DIFF_MIN - fileTreeWidth,
    );
  }

  function saveWidths() {
    localStorage.setItem("preflight:fileTreeWidth", String(fileTreeWidth));
    localStorage.setItem(
      "preflight:threadPanelWidth",
      String(threadPanelWidth),
    );
  }

  // Detect the topmost visible line in the diff for thread panel scroll sync
  let rafPending = false;
  function handleDiffScroll() {
    if (suppressSync || !mainEl || rafPending) return;
    rafPending = true;
    requestAnimationFrame(() => {
      rafPending = false;
      if (!mainEl || suppressSync) return;
      const rect = mainEl.getBoundingClientRect();
      const x = rect.left + 50;
      for (
        let y = rect.top + 5;
        y < Math.min(rect.top + 200, rect.bottom);
        y += 20
      ) {
        const el = document.elementFromPoint(x, y);
        if (!el) continue;
        const lineEl = (el.closest("[id^='L']") ??
          el.querySelector("[id^='L']")) as HTMLElement | null;
        if (lineEl) {
          const match = lineEl.id.match(/^L(\d+)$/);
          if (match) {
            visibleLine = parseInt(match[1], 10);
            return;
          }
        }
      }
    });
  }

  // ResizeObserver to clamp panes when the window shrinks
  $effect(() => {
    if (!containerEl) return;
    const observer = new ResizeObserver(() => clampWidths());
    observer.observe(containerEl);
    return () => observer.disconnect();
  });

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

  async function toggleStatus() {
    if (!review || statusUpdating) return;
    const newStatus = review.status === "Open" ? "Closed" : "Open";
    statusUpdating = true;
    try {
      await updateReviewStatus(reviewId, { status: newStatus });
      review = { ...review, status: newStatus };
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : "Failed to update status";
    } finally {
      statusUpdating = false;
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
        getReview(reviewId).then((r) => {
          review = r;
        });
        listFiles(reviewId, selectedRevision || undefined).then((f) => {
          files = f;
        });
      }),
      onEvent("comment_added", (event) => {
        if (event.review_id !== reviewId) return;
        if (selectedFile) loadThreads(selectedFile);
      }),
      onEvent("thread_status_changed", (event) => {
        if (event.review_id !== reviewId) return;
        if (selectedFile) loadThreads(selectedFile);
        getReview(reviewId).then((r) => {
          review = r;
        });
        listFiles(reviewId, selectedRevision || undefined).then((f) => {
          files = f;
        });
      }),
      onEvent("thread_acknowledged", (event) => {
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
      {#if review.open_thread_count > 0}
        <span class="text-xs text-status-open">
          {review.open_thread_count} unresolved
        </span>
      {/if}
      <button
        class="ml-auto text-xs px-2.5 py-1 rounded-md border transition-colors cursor-pointer
          {review.status === 'Open'
          ? 'border-border text-text-muted hover:text-badge-deleted hover:border-badge-deleted/50'
          : 'border-border text-text-muted hover:text-status-open hover:border-status-open/50'}"
        disabled={statusUpdating}
        onclick={toggleStatus}
      >
        {review.status === "Open" ? "Close review" : "Reopen review"}
      </button>
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
    <div
      class="flex flex-1 min-h-0"
      class:select-none={isDragging}
      bind:this={containerEl}
    >
      <!-- File tree -->
      <aside
        class="border-r border-border overflow-y-auto shrink-0"
        style:width="{fileTreeWidth}px"
      >
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

      <ResizeHandle
        side="left"
        onDragStart={() => (isDragging = true)}
        onDrag={(delta) => {
          fileTreeWidth = Math.max(
            FILE_TREE_MIN,
            Math.min(maxForFileTree(), fileTreeWidth + delta),
          );
        }}
        onDragEnd={() => {
          isDragging = false;
          saveWidths();
        }}
        onReset={() => {
          fileTreeWidth = FILE_TREE_DEFAULT;
          saveWidths();
        }}
      />

      <!-- Diff -->
      <main
        class="flex-1 overflow-auto min-w-0"
        bind:this={mainEl}
        onscroll={handleDiffScroll}
      >
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
        <ResizeHandle
          side="right"
          onDragStart={() => (isDragging = true)}
          onDrag={(delta) => {
            threadPanelWidth = Math.max(
              THREAD_PANEL_MIN,
              Math.min(maxForThreadPanel(), threadPanelWidth + delta),
            );
          }}
          onDragEnd={() => {
            isDragging = false;
            saveWidths();
          }}
          onReset={() => {
            threadPanelWidth = THREAD_PANEL_DEFAULT;
            saveWidths();
          }}
        />
        <aside
          class="border-l border-border overflow-y-auto shrink-0"
          style:width="{threadPanelWidth}px"
        >
          <ThreadPanel
            {threads}
            {highlightThreadId}
            {visibleLine}
            {diffLines}
            onNavigateToThread={(line) => {
              suppressSync = true;
              navigateToLine = line;
              setTimeout(() => {
                navigateToLine = null;
                suppressSync = false;
              }, 500);
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
