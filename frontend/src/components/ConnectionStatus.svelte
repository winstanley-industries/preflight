<script lang="ts">
  import { onStatus } from "../lib/ws";

  let status = $state<"connected" | "reconnecting" | "disconnected">(
    "disconnected",
  );

  $effect(() => {
    return onStatus((s) => {
      status = s;
    });
  });
</script>

<div
  class="fixed bottom-3 right-3 flex items-center gap-1.5 text-xs text-text-muted"
>
  <span
    class="w-2 h-2 rounded-full {status === 'connected'
      ? 'bg-green-500'
      : status === 'reconnecting'
        ? 'bg-yellow-500 animate-pulse'
        : 'bg-red-500'}"
  ></span>
  {#if status !== "connected"}
    <span
      >{status === "reconnecting" ? "Reconnecting\u2026" : "Disconnected"}</span
    >
  {/if}
</div>
