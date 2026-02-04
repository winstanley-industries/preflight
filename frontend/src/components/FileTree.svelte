<script lang="ts">
  import type { FileListEntry } from "../lib/types";

  interface Props {
    files: FileListEntry[];
    selectedFile: string | null;
    onSelect: (path: string) => void;
  }

  let { files, selectedFile, onSelect }: Props = $props();
</script>

<nav class="py-2">
  {#each files as file (file.path)}
    <button
      class="w-full text-left px-3 py-1 text-sm font-mono truncate cursor-pointer hover:bg-bg-hover"
      class:bg-bg-active={file.path === selectedFile}
      onclick={() => onSelect(file.path)}
    >
      {file.path}
    </button>
  {/each}
</nav>
