<script lang="ts">
  import type { ThreadResponse } from "../lib/types";

  interface Props {
    reviewId: string;
    threads: ThreadResponse[];
    onThreadsChanged: () => void;
  }

  let { reviewId: _reviewId, threads, onThreadsChanged: _onThreadsChanged }: Props = $props();
</script>

<div class="p-4">
  {#if threads.length === 0}
    <p class="text-text-muted text-sm">No threads on this file.</p>
  {:else}
    <p class="text-text-muted text-sm">{threads.length} threads — coming soon</p>
  {/if}
</div>
