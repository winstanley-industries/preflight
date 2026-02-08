<script lang="ts">
  import type { FileListEntry } from "../lib/types";
  import { buildFileTree } from "../lib/buildFileTree";
  import TreeNode from "./TreeNode.svelte";

  interface Props {
    files: FileListEntry[];
    selectedFile: string | null;
    onSelect: (path: string) => void;
  }

  let { files, selectedFile, onSelect }: Props = $props();

  let tree = $derived(buildFileTree(files));
</script>

<nav class="py-2">
  {#each tree as entry (entry.kind === "file" ? entry.path : entry.name)}
    <TreeNode {entry} depth={0} {selectedFile} {onSelect} />
  {/each}
</nav>
