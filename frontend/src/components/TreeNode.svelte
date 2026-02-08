<script lang="ts">
  import type { TreeEntry, FileNode, DirNode } from "../lib/buildFileTree";
  import type { FileStatus } from "../lib/types";
  import TreeNode from "./TreeNode.svelte";

  interface Props {
    entry: TreeEntry;
    depth: number;
    selectedFile: string | null;
    onSelect: (path: string) => void;
  }

  let { entry, depth, selectedFile, onSelect }: Props = $props();

  let expanded = $state(true);

  const statusIcon: Record<FileStatus, string> = {
    Added: "+",
    Modified: "\u25CF",
    Deleted: "\u2212",
    Renamed: "\u2192",
    Binary: "\u25C6",
  };

  const statusColor: Record<FileStatus, string> = {
    Added: "text-badge-added",
    Modified: "text-badge-modified",
    Deleted: "text-badge-deleted",
    Renamed: "text-badge-renamed",
    Binary: "text-badge-binary",
  };
</script>

{#if entry.kind === "dir"}
  {@const dir = entry as DirNode}
  <button
    class="w-full text-left py-1 text-sm font-mono flex items-center gap-1 cursor-pointer hover:bg-bg-hover transition-colors text-text-muted"
    style="padding-left: {depth * 16 + 8}px; padding-right: 12px;"
    aria-expanded={expanded}
    onclick={() => (expanded = !expanded)}
  >
    <span class="w-4 text-center shrink-0 text-xs">
      {expanded ? "\u25BC" : "\u25B6"}
    </span>
    <span class="truncate">{dir.name}</span>
    {#if dir.threadCount > 0}
      <span
        class="ml-auto shrink-0 text-xs px-1.5 py-0.5 rounded-full bg-status-open/15 text-status-open"
      >
        {dir.threadCount}
      </span>
    {/if}
  </button>
  {#if expanded}
    {#each dir.children as child (child.kind === "file" ? child.path : child.name)}
      <TreeNode entry={child} depth={depth + 1} {selectedFile} {onSelect} />
    {/each}
  {/if}
{:else}
  {@const file = entry as FileNode}
  <button
    class="w-full text-left py-1.5 text-sm font-mono truncate flex items-center gap-2 cursor-pointer hover:bg-bg-hover transition-colors"
    class:bg-bg-active={file.path === selectedFile}
    style="padding-left: {depth * 16 + 8}px; padding-right: 12px;"
    onclick={() => onSelect(file.path)}
    title={file.path}
  >
    <span class="w-4 text-center shrink-0 {statusColor[file.status]}">
      {statusIcon[file.status]}
    </span>
    <span class="truncate">{file.name}</span>
    {#if file.threadCount > 0}
      <span
        class="ml-auto shrink-0 text-xs px-1.5 py-0.5 rounded-full bg-status-open/15 text-status-open"
      >
        {file.threadCount}
      </span>
    {/if}
  </button>
{/if}
