<script lang="ts">
  import type { FileListEntry, FileStatus } from "../lib/types";

  interface Props {
    files: FileListEntry[];
    selectedFile: string | null;
    onSelect: (path: string) => void;
  }

  let { files, selectedFile, onSelect }: Props = $props();

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

<nav class="py-2">
  {#each files as file (file.path)}
    <button
      class="w-full text-left px-3 py-1.5 text-sm font-mono truncate flex items-center gap-2 cursor-pointer hover:bg-bg-hover transition-colors"
      class:bg-bg-active={file.path === selectedFile}
      onclick={() => onSelect(file.path)}
      title={file.path}
    >
      <span class="w-4 text-center shrink-0 {statusColor[file.status]}">
        {statusIcon[file.status]}
      </span>
      <span class="truncate">{file.path}</span>
    </button>
  {/each}
</nav>
