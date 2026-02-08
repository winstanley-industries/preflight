<script lang="ts">
  interface Props {
    title: string;
    message: string;
    confirmLabel?: string;
    onconfirm: () => void;
    oncancel: () => void;
  }

  let {
    title,
    message,
    confirmLabel = "Delete",
    onconfirm,
    oncancel,
  }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") oncancel();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
  role="button"
  tabindex="-1"
  onclick={oncancel}
  onkeydown={handleKeydown}
>
  <div
    class="bg-bg-surface border border-border rounded-lg p-6 max-w-sm w-full mx-4 shadow-lg"
    role="dialog"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <h2 class="text-lg font-semibold mb-2">{title}</h2>
    <p class="text-text-muted mb-6">{message}</p>
    <div class="flex justify-end gap-3">
      <button
        class="px-4 py-2 rounded-md text-sm text-text-muted hover:bg-bg-hover transition-colors cursor-pointer"
        onclick={oncancel}
      >
        Cancel
      </button>
      <button
        class="px-4 py-2 rounded-md text-sm bg-red-600 text-white hover:bg-red-500 transition-colors cursor-pointer"
        onclick={onconfirm}
      >
        {confirmLabel}
      </button>
    </div>
  </div>
</div>
