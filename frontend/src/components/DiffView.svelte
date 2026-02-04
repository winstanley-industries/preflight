<script lang="ts">
  import { getFileDiff } from "../lib/api";
  import type { FileDiffResponse, ThreadResponse } from "../lib/types";
  import InlineCommentForm from "./InlineCommentForm.svelte";

  interface Props {
    reviewId: string;
    filePath: string;
    threads: ThreadResponse[];
    onThreadCreated?: (threadId: string) => void;
  }

  let { reviewId, filePath, threads, onThreadCreated }: Props = $props();

  let diff = $state<FileDiffResponse | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);

  // Line selection state
  let selectionStart = $state<number | null>(null);
  let selectionEnd = $state<number | null>(null);
  let formOpen = $state(false);

  // Lines that have threads on them (for gutter indicators)
  let threadLines = $derived(
    new Set(
      threads.flatMap((t) => {
        const lines: number[] = [];
        for (let i = t.line_start; i <= t.line_end; i++) lines.push(i);
        return lines;
      }),
    ),
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

  // Compute where the form should appear (after the last selected line)
  let formLineNo = $derived(
    selectionStart !== null
      ? Math.max(selectionStart, selectionEnd ?? selectionStart)
      : null,
  );

  async function loadDiff(rid: string, path: string) {
    loading = true;
    error = null;
    closeForm();
    try {
      diff = await getFileDiff(rid, path);
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : "Failed to load diff";
      diff = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadDiff(reviewId, filePath);
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
  <div class="font-mono text-sm">
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
        {@const hasThread =
          line.new_line_no !== null && threadLines.has(line.new_line_no)}
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
              {#if hasThread}
                <span class="text-accent text-xs">&bull;</span>
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
          <span
            class="flex-1 px-2 whitespace-pre leading-6"
            class:text-diff-add-text={line.kind === "Added"}
            class:text-diff-remove-text={line.kind === "Removed"}
          >
            {line.content}
          </span>
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
            lineEnd={Math.max(selectionStart!, selectionEnd ?? selectionStart!)}
            onSubmit={handleThreadCreated}
            onCancel={closeForm}
          />
        {/if}
      {/each}
    {/each}
  </div>
{/if}
