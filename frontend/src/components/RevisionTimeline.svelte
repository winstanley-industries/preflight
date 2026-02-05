<script lang="ts">
  import type { RevisionResponse } from "../lib/types";

  interface Props {
    revisions: RevisionResponse[];
    selectedRevision: number;
    onSelect: (revisionNumber: number) => void;
    onRefresh: () => void;
  }

  let { revisions, selectedRevision, onSelect, onRefresh }: Props = $props();

  let refreshing = $state(false);

  function relativeTime(dateStr: string): string {
    const now = Date.now();
    const then = new Date(dateStr).getTime();
    const diffSec = Math.floor((now - then) / 1000);
    if (diffSec < 60) return "just now";
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    const diffDay = Math.floor(diffHr / 24);
    return `${diffDay}d ago`;
  }

  async function handleRefresh() {
    refreshing = true;
    try {
      onRefresh();
    } finally {
      // The parent controls the actual async flow; just show brief feedback
      setTimeout(() => {
        refreshing = false;
      }, 600);
    }
  }
</script>

<div
  class="flex items-center gap-1 px-4 py-1.5 border-b border-border bg-bg-surface shrink-0 overflow-x-auto"
>
  <span class="text-xs text-text-faint mr-1 shrink-0">Revisions</span>

  {#each revisions as rev (rev.id)}
    <button
      class="flex items-center gap-1.5 px-2 py-1 rounded text-xs transition-colors cursor-pointer shrink-0
        {rev.revision_number === selectedRevision
        ? 'bg-bg-active text-text'
        : 'text-text-muted hover:bg-bg-hover hover:text-text'}"
      onclick={() => onSelect(rev.revision_number)}
      title="{rev.trigger} \u2022 {relativeTime(rev.created_at)}{rev.message
        ? ` \u2022 ${rev.message}`
        : ''}"
    >
      <!-- Trigger icon -->
      <span class="text-text-faint">
        {#if rev.trigger === "Agent"}
          <svg
            class="w-3 h-3 inline-block"
            viewBox="0 0 16 16"
            fill="currentColor"
          >
            <path
              d="M4 6a4 4 0 1 1 8 0 4 4 0 0 1-8 0Zm4-2a2 2 0 1 0 0 4 2 2 0 0 0 0-4ZM3 13a3 3 0 0 1 3-3h4a3 3 0 0 1 3 3v1a1 1 0 0 1-2 0v-1a1 1 0 0 0-1-1H6a1 1 0 0 0-1 1v1a1 1 0 0 1-2 0v-1Z"
            />
            <path d="M12 3a1 1 0 0 1 1-1h1a1 1 0 1 1 0 2h-1a1 1 0 0 1-1-1Z" />
          </svg>
        {:else}
          <svg
            class="w-3 h-3 inline-block"
            viewBox="0 0 16 16"
            fill="currentColor"
          >
            <path
              d="M4 6a4 4 0 1 1 8 0 4 4 0 0 1-8 0Zm4-2a2 2 0 1 0 0 4 2 2 0 0 0 0-4ZM3 13a3 3 0 0 1 3-3h4a3 3 0 0 1 3 3v1a1 1 0 0 1-2 0v-1a1 1 0 0 0-1-1H6a1 1 0 0 0-1 1v1a1 1 0 0 1-2 0v-1Z"
            />
          </svg>
        {/if}
      </span>

      <!-- Revision number in a small circle -->
      <span
        class="inline-flex items-center justify-center w-5 h-5 rounded-full text-[10px] font-medium leading-none
          {rev.revision_number === selectedRevision
          ? 'bg-accent text-bg'
          : 'bg-bg-hover text-text-muted'}"
      >
        {rev.revision_number}
      </span>

      <!-- Relative time -->
      <span class="text-text-faint text-[10px]">
        {relativeTime(rev.created_at)}
      </span>
    </button>
  {/each}

  <!-- Refresh button -->
  <button
    class="ml-auto flex items-center gap-1 px-2 py-1 rounded text-xs text-text-muted hover:bg-bg-hover hover:text-text transition-colors cursor-pointer shrink-0"
    onclick={handleRefresh}
    disabled={refreshing}
    title="Snapshot current changes as a new revision"
  >
    <svg
      class="w-3.5 h-3.5 {refreshing ? 'animate-spin' : ''}"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
    >
      <path
        d="M2.5 8a5.5 5.5 0 0 1 9.21-4.07M13.5 8a5.5 5.5 0 0 1-9.21 4.07"
        stroke-linecap="round"
      />
      <path d="M11.5 1.5v3h3M4.5 14.5v-3h-3" stroke-linecap="round" />
    </svg>
    <span>Refresh</span>
  </button>
</div>
