import type { FileListEntry, FileStatus } from "./types";

export interface DirNode {
  kind: "dir";
  name: string;
  children: TreeEntry[];
  threadCount: number;
}

export interface FileNode {
  kind: "file";
  name: string;
  path: string;
  status: FileStatus;
  threadCount: number;
}

export type TreeEntry = DirNode | FileNode;

interface MutableDir {
  subdirs: Map<string, MutableDir>;
  files: FileListEntry[];
}

function newDir(): MutableDir {
  return { subdirs: new Map(), files: [] };
}

export function buildFileTree(entries: FileListEntry[]): TreeEntry[] {
  // Build intermediate tree
  const root = newDir();
  for (const entry of entries) {
    const parts = entry.path.split("/");
    parts.pop();
    let node = root;
    for (const part of parts) {
      if (!node.subdirs.has(part)) {
        node.subdirs.set(part, newDir());
      }
      node = node.subdirs.get(part)!;
    }
    node.files.push(entry);
  }

  // Convert to TreeEntry[], collapsing single-child chains
  function convert(dir: MutableDir, prefix: string): TreeEntry[] {
    const result: TreeEntry[] = [];

    const sortedDirs = [...dir.subdirs.entries()].sort((a, b) =>
      a[0].localeCompare(b[0]),
    );

    for (const [name, subdir] of sortedDirs) {
      const fullName = prefix ? `${prefix}/${name}` : name;
      // Collapse: single subdir, no files at this level
      if (subdir.subdirs.size === 1 && subdir.files.length === 0) {
        const collapsed = convert(subdir, fullName);
        result.push(...collapsed);
      } else {
        const children = convert(subdir, "");
        const threadCount = sumThreads(children);
        result.push({ kind: "dir", name: fullName, children, threadCount });
      }
    }

    const sortedFiles = [...dir.files].sort((a, b) => {
      const nameA = a.path.split("/").pop()!;
      const nameB = b.path.split("/").pop()!;
      return nameA.localeCompare(nameB);
    });

    for (const file of sortedFiles) {
      const name = file.path.split("/").pop()!;
      result.push({
        kind: "file",
        name,
        path: file.path,
        status: file.status,
        threadCount: file.open_thread_count,
      });
    }

    return result;
  }

  return convert(root, "");
}

function sumThreads(entries: TreeEntry[]): number {
  let total = 0;
  for (const entry of entries) {
    total += entry.threadCount;
  }
  return total;
}
