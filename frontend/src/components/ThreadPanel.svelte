<script lang="ts">
  import { addComment, updateThreadStatus } from "../lib/api";
  import type { ThreadResponse, ThreadOrigin } from "../lib/types";

  interface Props {
    threads: ThreadResponse[];
    highlightThreadId: string | null;
    diffLines: Set<number>;
    onThreadsChanged: () => void;
    onNavigateToThread: (lineStart: number) => void;
  }

  let {
    threads,
    highlightThreadId,
    diffLines,
    onThreadsChanged,
    onNavigateToThread,
  }: Props = $props();

  $effect(() => {
    if (highlightThreadId) {
      requestAnimationFrame(() => {
        const el = document.getElementById(`thread-${highlightThreadId}`);
        el?.scrollIntoView({ behavior: "smooth", block: "nearest" });
      });
    }
  });

  let replyTexts = $state<Record<string, string>>({});
  let submitting = $state<Record<string, boolean>>({});

  const originLabel: Record<ThreadOrigin, string> = {
    Comment: "Comment",
    ExplanationRequest: "Explanation Request",
    AgentExplanation: "Agent Explanation",
  };

  async function toggleStatus(thread: ThreadResponse) {
    const newStatus = thread.status === "Open" ? "Resolved" : "Open";
    await updateThreadStatus(thread.id, { status: newStatus });
    onThreadsChanged();
  }

  async function submitReply(threadId: string) {
    const body = replyTexts[threadId]?.trim();
    if (!body) return;
    submitting[threadId] = true;
    try {
      await addComment(threadId, { author_type: "Human", body });
      replyTexts[threadId] = "";
      onThreadsChanged();
    } finally {
      submitting[threadId] = false;
    }
  }

  function isThreadInDiff(thread: ThreadResponse): boolean {
    for (let i = thread.line_start; i <= thread.line_end; i++) {
      if (diffLines.has(i)) return true;
    }
    return false;
  }
</script>

<div class="py-3">
  <div class="px-4 pb-2">
    <h2 class="text-xs font-semibold text-text-muted uppercase tracking-wide">
      Threads
    </h2>
  </div>

  {#if threads.length === 0}
    <p class="px-4 text-sm text-text-muted">No threads on this file.</p>
  {:else}
    <div class="space-y-4">
      {#each threads as thread (thread.id)}
        <div id="thread-{thread.id}" class="border-b border-border pb-4 mx-4">
          <!-- Thread header -->
          <div class="flex items-center justify-between gap-2 mb-2">
            <button
              class="text-xs hover:underline cursor-pointer {isThreadInDiff(
                thread,
              )
                ? 'text-accent'
                : 'text-text-muted'}"
              title={isThreadInDiff(thread)
                ? undefined
                : "Opens full file view"}
              onclick={() => onNavigateToThread(thread.line_start)}
            >
              {#if !isThreadInDiff(thread)}<span class="mr-0.5">&rarr;</span
                >{/if}Lines {thread.line_start}&ndash;{thread.line_end}
            </button>
            <div class="flex items-center gap-2">
              <span
                class="text-xs px-1.5 py-0.5 rounded bg-bg-surface text-text-muted"
              >
                {originLabel[thread.origin]}
              </span>
              <span
                class="text-xs px-1.5 py-0.5 rounded {thread.status === 'Open'
                  ? 'bg-status-open/15 text-status-open'
                  : 'bg-bg-surface text-text-faint'}"
              >
                {thread.status}
              </span>
            </div>
          </div>

          <!-- Comments -->
          <div class="space-y-2">
            {#each thread.comments as comment (comment.id)}
              <div class="text-sm">
                <span
                  class="text-xs font-medium mr-1.5 {comment.author_type ===
                  'Agent'
                    ? 'text-badge-modified'
                    : 'text-text-muted'}"
                >
                  {comment.author_type}
                </span>
                <span class="text-text">{comment.body}</span>
              </div>
            {/each}
          </div>

          <!-- Reply box -->
          <div class="mt-2">
            <textarea
              class="w-full text-sm bg-bg-surface border border-border rounded px-2 py-1.5 text-text placeholder:text-text-faint focus:outline-none focus:border-accent resize-y"
              placeholder="Reply..."
              rows={2}
              bind:value={replyTexts[thread.id]}
              onkeydown={(e: KeyboardEvent) => {
                if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                  e.preventDefault();
                  submitReply(thread.id);
                }
              }}
            ></textarea>
            <div class="flex items-center gap-2 mt-1.5">
              <button
                class="text-sm px-2 py-1 bg-bg-surface border border-border rounded hover:bg-bg-hover transition-colors cursor-pointer disabled:opacity-50"
                disabled={submitting[thread.id] ||
                  !replyTexts[thread.id]?.trim()}
                onclick={() => submitReply(thread.id)}
              >
                Reply
              </button>
              <button
                class="text-sm px-2 py-1 rounded cursor-pointer transition-colors {thread.status ===
                'Open'
                  ? 'bg-status-open/15 text-status-open hover:bg-status-open/25'
                  : 'bg-bg-surface text-text-muted hover:bg-bg-hover'}"
                onclick={() => toggleStatus(thread)}
              >
                {thread.status === "Open" ? "Resolve" : "Reopen"}
              </button>
              <span class="text-xs text-text-faint ml-auto">⌘Enter</span>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
