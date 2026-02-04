<script lang="ts">
  import type { ThreadResponse } from "../lib/types";

  interface Props {
    reviewId: string;
    filePath: string;
    threads: ThreadResponse[];
  }

  let { reviewId: _reviewId, filePath, threads: _threads }: Props = $props();
</script>

<div class="p-4">
  <p class="text-text-muted font-mono text-sm">Diff for {filePath} — coming soon</p>
</div>
