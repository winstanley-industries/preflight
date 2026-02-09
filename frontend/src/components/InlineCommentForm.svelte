<script lang="ts">
  import { createThread } from "../lib/api";
  import type { ThreadOrigin } from "../lib/types";

  interface Props {
    reviewId: string;
    filePath: string;
    lineStart: number;
    lineEnd: number;
    onSubmit: (threadId: string) => void;
    onCancel: () => void;
  }

  let { reviewId, filePath, lineStart, lineEnd, onSubmit, onCancel }: Props =
    $props();

  let origin = $state<ThreadOrigin>("Comment");
  let body = $state("");
  let submitting = $state(false);
  let error = $state<string | null>(null);

  let inputEl: HTMLTextAreaElement | undefined = $state();

  const lineLabel = $derived(
    lineStart === lineEnd
      ? `Line ${lineStart}`
      : `Lines ${lineStart}\u2013${lineEnd}`,
  );

  const placeholder = $derived(
    origin === "ExplanationRequest"
      ? "What should be explained? (optional)"
      : "Add a comment...",
  );

  const canSubmit = $derived(
    !submitting && (origin === "ExplanationRequest" || !!body.trim()),
  );

  $effect(() => {
    inputEl?.focus();
  });

  async function submit() {
    const trimmed = body.trim();
    if (!canSubmit) return;
    submitting = true;
    error = null;
    try {
      const thread = await createThread(reviewId, {
        file_path: filePath,
        line_start: lineStart,
        line_end: lineEnd,
        origin,
        body: trimmed,
        author_type: "Human",
      });
      onSubmit(thread.id);
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : "Failed to create thread";
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onCancel();
    } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      submit();
    }
  }
</script>

<div class="bg-bg-surface border-y border-accent/30 px-4 py-3">
  <div class="flex items-center gap-3 mb-2">
    <span class="text-xs text-text-muted">{lineLabel}</span>
    <div class="flex gap-1">
      <button
        class="text-xs px-2 py-0.5 rounded cursor-pointer transition-colors {origin ===
        'Comment'
          ? 'bg-accent/20 text-accent'
          : 'bg-bg-hover text-text-muted hover:text-text'}"
        onclick={() => (origin = "Comment")}
      >
        Comment
      </button>
      <button
        class="text-xs px-2 py-0.5 rounded cursor-pointer transition-colors {origin ===
        'ExplanationRequest'
          ? 'bg-accent/20 text-accent'
          : 'bg-bg-hover text-text-muted hover:text-text'}"
        onclick={() => (origin = "ExplanationRequest")}
      >
        Request Explanation
      </button>
    </div>
  </div>
  <div>
    <textarea
      bind:this={inputEl}
      class="w-full text-sm bg-bg border border-border rounded px-2 py-1.5 text-text placeholder:text-text-faint focus:outline-none focus:border-accent resize-y"
      {placeholder}
      rows={3}
      bind:value={body}
      onkeydown={handleKeydown}
    ></textarea>
    <div class="flex items-center gap-2 mt-1.5">
      <button
        class="text-sm px-3 py-1 bg-accent text-bg rounded font-medium cursor-pointer transition-colors hover:bg-accent/80 disabled:opacity-50 disabled:cursor-not-allowed"
        disabled={!canSubmit}
        onclick={submit}
      >
        Submit
      </button>
      <button
        class="text-sm px-2 py-1 bg-bg-hover text-text-muted rounded cursor-pointer transition-colors hover:text-text"
        onclick={onCancel}
      >
        Cancel
      </button>
      <span class="text-xs text-text-faint ml-auto">⌘Enter to submit</span>
    </div>
  </div>
  {#if error}
    <p class="text-xs text-badge-deleted mt-1">{error}</p>
  {/if}
</div>
