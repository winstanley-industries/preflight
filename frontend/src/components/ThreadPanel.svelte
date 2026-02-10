<script lang="ts">
  import { addComment, updateThreadStatus, pokeThread } from "../lib/api";
  import type { ThreadResponse, ThreadOrigin } from "../lib/types";
  import { renderMarkdown } from "../lib/markdown";

  type StatusFilter = "Open" | "Resolved" | "All";
  type OriginFilter = ThreadOrigin | "All";
  type SortField = "location" | "newest" | "oldest";

  interface Props {
    threads: ThreadResponse[];
    highlightThreadId: string | null;
    visibleLine: number | null;
    diffLines: Set<number>;
    onThreadsChanged: () => void;
    onNavigateToThread: (lineStart: number) => void;
  }

  let {
    threads,
    highlightThreadId,
    visibleLine = null,
    diffLines,
    onThreadsChanged,
    onNavigateToThread,
  }: Props = $props();

  let statusFilter = $state<StatusFilter>("Open");
  let originFilter = $state<OriginFilter>("All");
  let sortField = $state<SortField>("location");

  function statusCount(status: StatusFilter): number {
    if (status === "All") return threads.length;
    return threads.filter((t) => t.status === status).length;
  }

  function filteredAndSortedThreads(): ThreadResponse[] {
    let filtered = threads;

    // Status filter
    if (statusFilter !== "All") {
      filtered = filtered.filter((t) => t.status === statusFilter);
    }

    // Origin filter
    if (originFilter !== "All") {
      filtered = filtered.filter((t) => t.origin === originFilter);
    }

    // Sort
    return [...filtered].sort((a, b) => {
      switch (sortField) {
        case "location":
          return a.line_start - b.line_start;
        case "newest":
          return (
            new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
          );
        case "oldest":
          return (
            new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
          );
      }
    });
  }

  $effect(() => {
    if (highlightThreadId) {
      requestAnimationFrame(() => {
        const el = document.getElementById(`thread-${highlightThreadId}`);
        el?.scrollIntoView({ behavior: "smooth", block: "nearest" });
      });
    }
  });

  // Scroll-sync: when sorted by location, follow the diff scroll
  let lastSyncedThreadId: string | null = null;
  $effect(() => {
    if (sortField !== "location" || visibleLine == null) return;
    const sorted = filteredAndSortedThreads();
    if (sorted.length === 0) return;
    // Find first thread that covers or follows the visible line
    const target =
      sorted.find((t) => t.line_end >= visibleLine) ??
      sorted[sorted.length - 1];
    if (!target || target.id === lastSyncedThreadId) return;
    lastSyncedThreadId = target.id;
    requestAnimationFrame(() => {
      const el = document.getElementById(`thread-${target.id}`);
      el?.scrollIntoView({ behavior: "smooth", block: "nearest" });
    });
  });

  let replyTexts = $state<Record<string, string>>({});
  let submitting = $state<Record<string, boolean>>({});
  let poking = $state<Record<string, boolean>>({});

  function lastCommentIsHuman(thread: ThreadResponse): boolean {
    const last = thread.comments[thread.comments.length - 1];
    return last?.author_type === "Human";
  }

  async function handlePoke(threadId: string) {
    poking[threadId] = true;
    try {
      await pokeThread(threadId);
    } finally {
      setTimeout(() => {
        poking[threadId] = false;
      }, 2000);
    }
  }

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
    <!-- Filter bar -->
    <div class="px-4 pb-2 space-y-1.5">
      <div class="flex items-center gap-1 text-xs">
        {#each ["Open", "Resolved", "All"] as status (status)}
          <button
            class="px-2 py-0.5 rounded transition-colors cursor-pointer {statusFilter ===
            status
              ? 'bg-bg-hover text-text'
              : 'text-text-muted hover:text-text'}"
            onclick={() => (statusFilter = status as StatusFilter)}
          >
            {status}
            <span class="text-text-faint ml-0.5"
              >{statusCount(status as StatusFilter)}</span
            >
          </button>
        {/each}
      </div>
      <div class="flex items-center gap-2">
        <select
          aria-label="Filter by origin"
          bind:value={originFilter}
          class="flex-1 bg-bg-surface text-xs text-text-muted border border-border rounded px-1.5 py-0.5 outline-none cursor-pointer focus:border-text-muted transition-colors"
        >
          <option value="All">All origins</option>
          <option value="Comment">Comment</option>
          <option value="ExplanationRequest">Explanation Request</option>
          <option value="AgentExplanation">Agent Explanation</option>
        </select>
        <select
          aria-label="Sort threads"
          bind:value={sortField}
          class="flex-1 bg-bg-surface text-xs text-text-muted border border-border rounded px-1.5 py-0.5 outline-none cursor-pointer focus:border-text-muted transition-colors"
        >
          <option value="location">By location</option>
          <option value="newest">Newest</option>
          <option value="oldest">Oldest</option>
        </select>
      </div>
    </div>

    {#if filteredAndSortedThreads().length === 0}
      <p class="px-4 text-sm text-text-muted">No matching threads.</p>
    {:else}
      <div class="space-y-4">
        {#each filteredAndSortedThreads() as thread (thread.id)}
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

            <!-- Agent activity status -->
            {#if thread.agent_status === "Seen"}
              <div
                class="flex items-center gap-1.5 text-xs text-text-faint mb-2"
              >
                <svg
                  class="w-3.5 h-3.5"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                  <circle cx="12" cy="12" r="3" />
                </svg>
                Agent has seen this
              </div>
            {:else if thread.agent_status === "Researching"}
              <div
                class="flex items-center gap-1.5 text-xs text-text-muted mb-2"
              >
                <svg
                  class="w-3.5 h-3.5"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <circle cx="11" cy="11" r="8" />
                  <path d="M21 21l-4.35-4.35" />
                </svg>
                Agent is researching&hellip;
              </div>
            {:else if thread.agent_status === "Working"}
              <div class="flex items-center gap-1.5 text-xs text-accent mb-2">
                <svg
                  class="w-3.5 h-3.5 animate-spin"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path
                    d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"
                  />
                </svg>
                Agent is working on this&hellip;
              </div>
            {:else if thread.status === "Open" && lastCommentIsHuman(thread)}
              <div class="mb-2">
                <button
                  class="text-xs px-2 py-0.5 rounded border border-border text-text-faint hover:text-text hover:bg-bg-hover transition-colors cursor-pointer disabled:opacity-50"
                  disabled={poking[thread.id]}
                  onclick={() => handlePoke(thread.id)}
                >
                  {poking[thread.id] ? "Nudged!" : "Nudge agent"}
                </button>
              </div>
            {/if}

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
                  <div class="markdown-body text-text">
                    <!-- eslint-disable-next-line svelte/no-at-html-tags -- sanitized by DOMPurify -->
                    {@html renderMarkdown(comment.body)}
                  </div>
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
  {/if}
</div>
