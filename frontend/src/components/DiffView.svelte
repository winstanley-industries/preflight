<script lang="ts">
  import { getFileDiff } from "../lib/api";
  import type { FileDiffResponse, ThreadResponse } from "../lib/types";

  interface Props {
    reviewId: string;
    filePath: string;
    threads: ThreadResponse[];
  }

  let { reviewId, filePath, threads }: Props = $props();

  let diff = $state<FileDiffResponse | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);

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

  async function loadDiff(rid: string, path: string) {
    loading = true;
    error = null;
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
    {#each diff.hunks as hunk}
      <!-- Hunk header -->
      <div class="px-4 py-1 text-text-faint bg-bg-surface text-xs select-none border-y border-border">
        @@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count} @@
        {#if hunk.context}
          <span class="ml-2">{hunk.context}</span>
        {/if}
      </div>

      <!-- Diff lines -->
      {#each hunk.lines as line}
        <div
          class="flex hover:brightness-125 transition-[filter]"
          class:bg-diff-add-bg={line.kind === "Added"}
          class:bg-diff-remove-bg={line.kind === "Removed"}
          id={line.new_line_no ? `L${line.new_line_no}` : undefined}
        >
          <!-- Gutter: old line number -->
          <span class="w-12 shrink-0 text-right pr-2 select-none text-text-faint text-xs leading-6">
            {line.old_line_no ?? ""}
          </span>
          <!-- Gutter: new line number -->
          <span class="w-12 shrink-0 text-right pr-2 select-none text-text-faint text-xs leading-6">
            {line.new_line_no ?? ""}
          </span>
          <!-- Thread indicator -->
          <span class="w-4 shrink-0 text-center select-none leading-6">
            {#if line.new_line_no && threadLines.has(line.new_line_no)}
              <span class="text-accent text-xs">&bull;</span>
            {/if}
          </span>
          <!-- Line content -->
          <span
            class="flex-1 px-2 whitespace-pre leading-6"
            class:text-diff-add-text={line.kind === "Added"}
            class:text-diff-remove-text={line.kind === "Removed"}
          >
            {line.content}
          </span>
        </div>
      {/each}
    {/each}
  </div>
{/if}
