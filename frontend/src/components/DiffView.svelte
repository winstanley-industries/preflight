<script lang="ts">
  import { tick } from "svelte";
  import { SvelteMap } from "svelte/reactivity";
  import { getFileDiff, getFileInterdiff, getFileContent } from "../lib/api";
  import type {
    FileDiffResponse,
    FileContentResponse,
    FileStatus,
    ThreadResponse,
  } from "../lib/types";
  import InlineCommentForm from "./InlineCommentForm.svelte";

  interface Props {
    reviewId: string;
    filePath: string;
    threads: ThreadResponse[];
    fileStatus: FileStatus;
    revision?: number;
    interdiff?: { from: number; to: number } | null;
    navigateToLine?: number | null;
    onThreadCreated?: (threadId: string) => void;
    onDiffLinesKnown?: (lines: Set<number>) => void;
  }

  let {
    reviewId,
    filePath,
    threads,
    fileStatus,
    revision,
    interdiff = null,
    navigateToLine = null,
    onThreadCreated,
    onDiffLinesKnown,
  }: Props = $props();

  let diff = $state<FileDiffResponse | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);

  let viewMode = $state<"diff" | "file">("diff");
  let fileContent = $state<FileContentResponse | null>(null);
  let fileLoading = $state(false);
  let fileVersion = $state<"new" | "old">("new");
  let wordWrap = $state(true);
  let contentWs = $derived(
    wordWrap ? "whitespace-pre-wrap break-words" : "whitespace-pre",
  );
  let containerFit = $derived(wordWrap ? "" : "w-fit");

  // Line selection state
  let selectionStart = $state<number | null>(null);
  let selectionEnd = $state<number | null>(null);
  let formOpen = $state(false);

  // Lines that have threads on them (for gutter indicators)
  // Maps line number to thread status — Open wins over Resolved
  let threadLineStatus = $derived(
    (() => {
      const map = new SvelteMap<number, "Open" | "Resolved">();
      for (const t of threads) {
        for (let i = t.line_start; i <= t.line_end; i++) {
          const current = map.get(i);
          if (!current || t.status === "Open") {
            map.set(i, t.status);
          }
        }
      }
      return map;
    })(),
  );

  function isLineSelected(lineNo: number | null): boolean {
    if (lineNo === null || selectionStart === null) return false;
    const end = selectionEnd ?? selectionStart;
    const lo = Math.min(selectionStart, end);
    const hi = Math.max(selectionStart, end);
    return lineNo >= lo && lineNo <= hi;
  }

  function handleGutterClick(lineNo: number, e: MouseEvent) {
    if (e.shiftKey && selectionStart !== null) {
      // Extend selection
      selectionEnd = lineNo;
      formOpen = true;
    } else {
      // New selection
      selectionStart = lineNo;
      selectionEnd = lineNo;
      formOpen = true;
    }
  }

  function closeForm() {
    formOpen = false;
    selectionStart = null;
    selectionEnd = null;
  }

  function handleThreadCreated(threadId: string) {
    closeForm();
    onThreadCreated?.(threadId);
  }

  // Lines changed in the diff, for highlighting in file view
  let changedLines = $derived(
    new Set(
      diff?.hunks.flatMap((hunk) =>
        hunk.lines
          .filter((line) =>
            fileVersion === "new"
              ? line.kind === "Added" && line.new_line_no !== null
              : line.kind === "Removed" && line.old_line_no !== null,
          )
          .map((line) =>
            fileVersion === "new" ? line.new_line_no! : line.old_line_no!,
          ),
      ) ?? [],
    ),
  );

  // Compute where the form should appear (after the last selected line)
  let formLineNo = $derived(
    selectionStart !== null
      ? Math.max(selectionStart, selectionEnd ?? selectionStart)
      : null,
  );

  async function loadFileContent(
    rid: string,
    path: string,
    version: "old" | "new" = "new",
  ) {
    fileLoading = true;
    try {
      fileContent = await getFileContent(rid, path, version);
    } catch {
      fileContent = null;
    } finally {
      fileLoading = false;
    }
  }

  async function loadDiff(
    rid: string,
    path: string,
    rev?: number,
    inter?: { from: number; to: number } | null,
  ) {
    loading = true;
    error = null;
    closeForm();
    viewMode = "diff";
    fileContent = null;
    fileVersion = "new";
    try {
      if (inter) {
        diff = await getFileInterdiff(rid, path, inter.from, inter.to);
      } else {
        diff = await getFileDiff(rid, path, rev);
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : "Failed to load diff";
      diff = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadDiff(reviewId, filePath, revision, interdiff);
  });

  // Set of all new-side line numbers present in the diff
  let diffLineNumbers = $derived(
    new Set(
      diff?.hunks.flatMap((hunk) =>
        hunk.lines
          .filter((line) => line.new_line_no !== null)
          .map((line) => line.new_line_no!),
      ) ?? [],
    ),
  );

  // Report diff line numbers to parent when they change
  $effect(() => {
    if (diff) {
      onDiffLinesKnown?.(diffLineNumbers);
    }
  });

  // Navigate to a specific line when requested
  $effect(() => {
    if (navigateToLine == null) return;
    const target = navigateToLine;

    if (diffLineNumbers.has(target)) {
      // Line is in the diff — scroll to it
      requestAnimationFrame(() => {
        const el = document.getElementById(`L${target}`);
        el?.scrollIntoView({ behavior: "smooth", block: "center" });
      });
    } else {
      // Line is outside the diff — switch to file view
      viewMode = "file";
      if (!fileContent || fileVersion !== "new") {
        fileVersion = "new";
        loadFileContent(reviewId, filePath, "new").then(async () => {
          await tick();
          const el = document.getElementById(`L${target}`);
          el?.scrollIntoView({ behavior: "smooth", block: "center" });
        });
      } else {
        tick().then(() => {
          const el = document.getElementById(`L${target}`);
          el?.scrollIntoView({ behavior: "smooth", block: "center" });
        });
      }
    }
  });
</script>

{#if loading}
  <div class="p-4">
    <p class="text-text-muted text-sm">Loading diff...</p>
  </div>
{:else if error}
  <div class="p-4">
    <p class="text-badge-deleted text-sm">{error}</p>
  </div>
{:else if diff}
  <!-- View mode toggle -->
  <div
    class="flex items-center gap-2 px-4 py-1.5 border-b border-border bg-bg-surface"
  >
    <button
      class="text-xs px-2 py-0.5 rounded cursor-pointer {viewMode === 'diff'
        ? 'bg-bg-active text-text'
        : 'text-text-muted hover:text-text'}"
      onclick={() => {
        viewMode = "diff";
        fileVersion = "new";
      }}>Diff</button
    >
    <button
      class="text-xs px-2 py-0.5 rounded cursor-pointer {viewMode === 'file'
        ? 'bg-bg-active text-text'
        : 'text-text-muted hover:text-text'}"
      onclick={() => {
        viewMode = "file";
        if (!fileContent || fileVersion !== "new") {
          fileVersion = "new";
          loadFileContent(reviewId, filePath, "new");
        }
      }}>File</button
    >

    <span class="mx-1 text-border">|</span>

    <button
      class="text-xs px-2 py-0.5 rounded cursor-pointer {wordWrap
        ? 'bg-bg-active text-text'
        : 'text-text-muted hover:text-text'}"
      onclick={() => (wordWrap = !wordWrap)}
      title={wordWrap ? "Disable word wrap" : "Enable word wrap"}>Wrap</button
    >
  </div>

  {#if viewMode === "file"}
    <div
      class="flex items-center gap-2 px-4 py-1 border-b border-border bg-bg-surface"
    >
      <button
        class="text-xs px-2 py-0.5 rounded {fileVersion === 'new'
          ? 'bg-bg-active text-text'
          : fileStatus === 'Deleted'
            ? 'text-text-faint cursor-not-allowed'
            : 'text-text-muted hover:text-text cursor-pointer'}"
        disabled={fileStatus === "Deleted"}
        title={fileStatus === "Deleted"
          ? "No new version for deleted files"
          : undefined}
        onclick={() => {
          fileVersion = "new";
          loadFileContent(reviewId, filePath, "new");
        }}>New</button
      >
      <button
        class="text-xs px-2 py-0.5 rounded {fileVersion === 'old'
          ? 'bg-bg-active text-text'
          : fileStatus === 'Added'
            ? 'text-text-faint cursor-not-allowed'
            : 'text-text-muted hover:text-text cursor-pointer'}"
        disabled={fileStatus === "Added"}
        title={fileStatus === "Added"
          ? "No old version for new files"
          : undefined}
        onclick={() => {
          fileVersion = "old";
          loadFileContent(reviewId, filePath, "old");
        }}>Old</button
      >
    </div>
  {/if}

  {#if viewMode === "diff"}
    <div class="font-mono text-sm min-w-full {containerFit}">
      {#each diff.hunks as hunk, hunkIdx (hunkIdx)}
        <!-- Hunk header -->
        <div
          class="px-4 py-1 text-text-faint bg-bg-surface text-xs select-none border-y border-border"
        >
          @@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count}
          @@
          {#if hunk.context}
            <span class="ml-2">{hunk.context}</span>
          {/if}
        </div>

        <!-- Diff lines -->
        {#each hunk.lines as line, lineIdx (lineIdx)}
          {@const commentable = line.new_line_no !== null}
          {@const threadStatus =
            line.new_line_no !== null
              ? threadLineStatus.get(line.new_line_no)
              : undefined}
          {@const selected = isLineSelected(line.new_line_no)}
          <div
            class="group flex hover:brightness-125 transition-[filter] {selected
              ? 'bg-accent/10'
              : ''}"
            class:bg-diff-add-bg={line.kind === "Added" && !selected}
            class:bg-diff-remove-bg={line.kind === "Removed" && !selected}
            id={line.new_line_no ? `L${line.new_line_no}` : undefined}
          >
            <!-- Gutter: old line number -->
            <span
              class="w-12 shrink-0 text-right pr-2 select-none text-text-faint text-xs leading-6"
            >
              {line.old_line_no ?? ""}
            </span>
            <!-- Gutter: new line number -->
            <span
              class="w-12 shrink-0 text-right pr-2 select-none text-text-faint text-xs leading-6"
            >
              {line.new_line_no ?? ""}
            </span>
            <!-- Thread indicator / add button -->
            {#if commentable}
              <button
                class="w-6 shrink-0 text-center select-none leading-6 cursor-pointer"
                onclick={(e: MouseEvent) =>
                  handleGutterClick(line.new_line_no!, e)}
              >
                {#if threadStatus}
                  <span
                    class="text-xs {threadStatus === 'Open'
                      ? 'text-status-open'
                      : 'text-text-faint'}">&bull;</span
                  >
                {:else}
                  <span
                    class="text-accent text-xs opacity-0 group-hover:opacity-100 transition-opacity"
                    >+</span
                  >
                {/if}
              </button>
            {:else}
              <span class="w-6 shrink-0"></span>
            {/if}
            <!-- Line content -->
            {#if line.highlighted}
              <!-- eslint-disable svelte/no-at-html-tags -->
              <span
                class="flex-1 px-2 leading-6 {contentWs}"
                class:text-diff-add-text={line.kind === "Added"}
                class:text-diff-remove-text={line.kind === "Removed"}
                >{@html line.highlighted}</span
              >
              <!-- eslint-enable svelte/no-at-html-tags -->
            {:else}
              <span
                class="flex-1 px-2 leading-6 {contentWs}"
                class:text-diff-add-text={line.kind === "Added"}
                class:text-diff-remove-text={line.kind === "Removed"}
              >
                {line.content}
              </span>
            {/if}
          </div>

          <!-- Inline comment form (after the last selected line) -->
          {#if formOpen && line.new_line_no === formLineNo}
            <InlineCommentForm
              {reviewId}
              {filePath}
              lineStart={Math.min(
                selectionStart!,
                selectionEnd ?? selectionStart!,
              )}
              lineEnd={Math.max(
                selectionStart!,
                selectionEnd ?? selectionStart!,
              )}
              onSubmit={handleThreadCreated}
              onCancel={closeForm}
            />
          {/if}
        {/each}
      {/each}
    </div>
  {:else if fileLoading}
    <div class="p-4">
      <p class="text-text-muted text-sm">Loading file...</p>
    </div>
  {:else if fileContent}
    <div class="font-mono text-sm min-w-full {containerFit}">
      {#each fileContent.lines as line (line.line_no)}
        {@const threadStatus = threadLineStatus.get(line.line_no)}
        {@const selected = isLineSelected(line.line_no)}
        <div
          class="group flex hover:brightness-125 transition-[filter] {selected
            ? 'bg-accent/10'
            : ''}"
          class:bg-diff-add-bg={fileVersion === "new" &&
            changedLines.has(line.line_no) &&
            !selected}
          class:bg-diff-remove-bg={fileVersion === "old" &&
            changedLines.has(line.line_no) &&
            !selected}
          id={`L${line.line_no}`}
        >
          <span
            class="w-12 shrink-0 text-right pr-2 select-none text-text-faint text-xs leading-6"
          >
            {line.line_no}
          </span>
          <button
            class="w-6 shrink-0 text-center select-none leading-6 cursor-pointer"
            onclick={(e: MouseEvent) => handleGutterClick(line.line_no, e)}
          >
            {#if threadStatus}
              <span
                class="text-xs {threadStatus === 'Open'
                  ? 'text-status-open'
                  : 'text-text-faint'}">&bull;</span
              >
            {:else}
              <span
                class="text-accent text-xs opacity-0 group-hover:opacity-100 transition-opacity"
                >+</span
              >
            {/if}
          </button>
          {#if line.highlighted}
            <!-- eslint-disable svelte/no-at-html-tags -->
            <span class="flex-1 px-2 leading-6 {contentWs}"
              >{@html line.highlighted}</span
            >
            <!-- eslint-enable svelte/no-at-html-tags -->
          {:else}
            <span class="flex-1 px-2 leading-6 {contentWs}">{line.content}</span
            >
          {/if}
        </div>

        <!-- Inline comment form (after the last selected line) -->
        {#if formOpen && line.line_no === formLineNo}
          <InlineCommentForm
            {reviewId}
            {filePath}
            lineStart={Math.min(
              selectionStart!,
              selectionEnd ?? selectionStart!,
            )}
            lineEnd={Math.max(selectionStart!, selectionEnd ?? selectionStart!)}
            onSubmit={handleThreadCreated}
            onCancel={closeForm}
          />
        {/if}
      {/each}
    </div>
  {/if}
{/if}
