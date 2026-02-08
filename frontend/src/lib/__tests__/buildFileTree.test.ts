import { describe, it, expect } from "vitest";
import { buildFileTree } from "../buildFileTree";
import type { FileListEntry } from "../types";

function makeFile(
  path: string,
  status: "Added" | "Modified" | "Deleted" | "Renamed" | "Binary" = "Modified",
  open_thread_count = 0,
): FileListEntry {
  return { path, status, thread_count: open_thread_count, open_thread_count };
}

describe("buildFileTree", () => {
  it("returns empty array for empty input", () => {
    expect(buildFileTree([])).toEqual([]);
  });

  it("returns a single file at root level", () => {
    const result = buildFileTree([makeFile("README.md")]);
    expect(result).toEqual([
      {
        kind: "file",
        name: "README.md",
        path: "README.md",
        status: "Modified",
        threadCount: 0,
      },
    ]);
  });

  it("groups files under a directory", () => {
    const result = buildFileTree([
      makeFile("src/main.ts"),
      makeFile("src/utils.ts"),
    ]);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("dir");
    if (result[0].kind === "dir") {
      expect(result[0].name).toBe("src");
      expect(result[0].children).toHaveLength(2);
      expect(result[0].children[0]).toMatchObject({
        kind: "file",
        name: "main.ts",
      });
    }
  });

  it("collapses single-child directory chains", () => {
    const result = buildFileTree([
      makeFile("crates/server/src/main.rs"),
      makeFile("crates/server/src/routes.rs"),
    ]);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("dir");
    if (result[0].kind === "dir") {
      expect(result[0].name).toBe("crates/server/src");
      expect(result[0].children).toHaveLength(2);
    }
  });

  it("does not collapse when a directory has multiple children", () => {
    const result = buildFileTree([
      makeFile("crates/core/lib.rs"),
      makeFile("crates/server/main.rs"),
    ]);
    expect(result).toHaveLength(1);
    expect(result[0].kind).toBe("dir");
    if (result[0].kind === "dir") {
      expect(result[0].name).toBe("crates");
      expect(result[0].children).toHaveLength(2);
      expect(result[0].children[0]).toMatchObject({
        kind: "dir",
        name: "core",
      });
      expect(result[0].children[1]).toMatchObject({
        kind: "dir",
        name: "server",
      });
    }
  });

  it("aggregates thread counts in directories", () => {
    const result = buildFileTree([
      makeFile("src/a.ts", "Modified", 2),
      makeFile("src/b.ts", "Added", 3),
    ]);
    expect(result[0].kind).toBe("dir");
    if (result[0].kind === "dir") {
      expect(result[0].threadCount).toBe(5);
    }
  });

  it("sorts directories before files, alphabetically within each", () => {
    const result = buildFileTree([
      makeFile("src/z.ts"),
      makeFile("src/lib/a.ts"),
      makeFile("src/a.ts"),
    ]);
    expect(result[0].kind).toBe("dir");
    if (result[0].kind === "dir") {
      const names = result[0].children.map((c) => c.name);
      expect(names).toEqual(["lib", "a.ts", "z.ts"]);
    }
  });
});
