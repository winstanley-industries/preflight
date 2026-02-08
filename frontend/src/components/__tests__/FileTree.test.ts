import { render, screen, cleanup } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { FileListEntry } from "../../lib/types";
import FileTree from "../FileTree.svelte";

function makeFile(
  path: string,
  overrides: Partial<FileListEntry> = {},
): FileListEntry {
  return {
    path,
    status: "Modified",
    thread_count: 0,
    open_thread_count: 0,
    ...overrides,
  };
}

function renderTree(
  files: FileListEntry[],
  overrides: Record<string, unknown> = {},
) {
  return render(FileTree, {
    props: {
      files,
      selectedFile: null,
      onSelect: vi.fn(),
      ...overrides,
    },
  });
}

describe("FileTree", () => {
  beforeEach(() => {
    cleanup();
  });

  it("renders files with correct filenames (not full paths)", () => {
    renderTree([makeFile("src/main.ts"), makeFile("src/utils.ts")]);
    expect(screen.getByText("main.ts")).toBeInTheDocument();
    expect(screen.getByText("utils.ts")).toBeInTheDocument();
  });

  it("renders directory names", () => {
    renderTree([makeFile("src/main.ts")]);
    expect(screen.getByText("src")).toBeInTheDocument();
  });

  it("clicking a file calls onSelect with full path", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    renderTree([makeFile("src/main.ts")], { onSelect });
    await user.click(screen.getByText("main.ts"));
    expect(onSelect).toHaveBeenCalledWith("src/main.ts");
  });

  it("clicking a directory collapses its children and clicking again expands them", async () => {
    const user = userEvent.setup();
    renderTree([makeFile("src/main.ts"), makeFile("src/utils.ts")]);

    // Both files visible initially (expanded)
    expect(screen.getByText("main.ts")).toBeInTheDocument();
    expect(screen.getByText("utils.ts")).toBeInTheDocument();

    // Click the directory to collapse
    await user.click(screen.getByText("src"));
    expect(screen.queryByText("main.ts")).not.toBeInTheDocument();
    expect(screen.queryByText("utils.ts")).not.toBeInTheDocument();

    // Click again to expand
    await user.click(screen.getByText("src"));
    expect(screen.getByText("main.ts")).toBeInTheDocument();
    expect(screen.getByText("utils.ts")).toBeInTheDocument();
  });

  it('shows "+" icon for Added files and "●" icon for Modified files', () => {
    renderTree([
      makeFile("src/added.ts", { status: "Added" }),
      makeFile("src/modified.ts", { status: "Modified" }),
    ]);
    const buttons = screen.getAllByRole("button");
    // File buttons have title attributes with the full path
    const addedButton = buttons.find((b) => b.title === "src/added.ts")!;
    const modifiedButton = buttons.find((b) => b.title === "src/modified.ts")!;

    expect(addedButton.textContent).toContain("+");
    expect(modifiedButton.textContent).toContain("\u25CF");
  });

  it("shows thread count badge when open_thread_count > 0", () => {
    renderTree([makeFile("src/main.ts", { open_thread_count: 3 })]);
    const fileButton = screen
      .getAllByRole("button")
      .find((b) => b.title === "src/main.ts")!;
    const badge = fileButton.querySelector(
      ".bg-status-open\\/15",
    ) as HTMLElement;
    expect(badge).not.toBeNull();
    expect(badge.textContent!.trim()).toBe("3");
  });

  it("does not show thread count badge when open_thread_count is 0", () => {
    renderTree([makeFile("src/main.ts", { open_thread_count: 0 })]);
    // The file button should exist but not contain a badge
    const fileButton = screen
      .getAllByRole("button")
      .find((b) => b.title === "src/main.ts")!;
    // The badge would be a span with a number; the button should only have the status icon and filename
    const spans = fileButton.querySelectorAll("span");
    // Should have exactly 2 spans: status icon + filename (no badge span)
    expect(spans.length).toBe(2);
  });

  it("selected file gets bg-bg-active class", () => {
    renderTree([makeFile("src/main.ts")], { selectedFile: "src/main.ts" });
    const fileButton = screen
      .getAllByRole("button")
      .find((b) => b.title === "src/main.ts")!;
    expect(fileButton.className).toContain("bg-bg-active");
  });

  it("file buttons have title attribute with full path", () => {
    renderTree([makeFile("src/lib/helpers.ts")]);
    const fileButton = screen
      .getAllByRole("button")
      .find((b) => b.title === "src/lib/helpers.ts");
    expect(fileButton).toBeDefined();
    expect(fileButton!.title).toBe("src/lib/helpers.ts");
  });
});
