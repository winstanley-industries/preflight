import { render, cleanup } from "@testing-library/svelte";
import { describe, it, expect, vi, beforeEach } from "vitest";
import ResizeHandle from "../ResizeHandle.svelte";

// jsdom doesn't implement pointer capture APIs
beforeEach(() => {
  HTMLElement.prototype.setPointerCapture = vi.fn();
  HTMLElement.prototype.releasePointerCapture = vi.fn();
});

function getWrapper(container: HTMLElement): HTMLElement {
  // The outer wrapper div has the pointer event handlers
  const el = container.querySelector(".relative");
  if (!el) throw new Error("Could not find resize wrapper");
  return el as HTMLElement;
}

function getHitArea(container: HTMLElement): HTMLElement {
  const el = container.querySelector(".cursor-col-resize");
  if (!el) throw new Error("Could not find resize hit area");
  return el as HTMLElement;
}

function firePointer(
  el: HTMLElement,
  type: "pointerdown" | "pointermove" | "pointerup",
  clientX: number,
) {
  const event = new PointerEvent(type, {
    clientX,
    bubbles: true,
    pointerId: 1,
  });
  el.dispatchEvent(event);
}

describe("ResizeHandle", () => {
  beforeEach(() => {
    cleanup();
  });

  it("calls onDragStart on pointer down", () => {
    const onDragStart = vi.fn();
    const { container } = render(ResizeHandle, {
      props: { side: "left", onDrag: vi.fn(), onDragStart },
    });
    const wrapper = getWrapper(container);

    firePointer(wrapper, "pointerdown", 100);
    expect(onDragStart).toHaveBeenCalledOnce();
  });

  it("calls onDrag with positive delta for left side when moving right", () => {
    const onDrag = vi.fn();
    const { container } = render(ResizeHandle, {
      props: { side: "left", onDrag },
    });
    const wrapper = getWrapper(container);

    firePointer(wrapper, "pointerdown", 100);
    firePointer(wrapper, "pointermove", 120);
    expect(onDrag).toHaveBeenCalledWith(20);
  });

  it("calls onDrag with negative delta for right side when moving right", () => {
    const onDrag = vi.fn();
    const { container } = render(ResizeHandle, {
      props: { side: "right", onDrag },
    });
    const wrapper = getWrapper(container);

    firePointer(wrapper, "pointerdown", 100);
    firePointer(wrapper, "pointermove", 120);
    expect(onDrag).toHaveBeenCalledWith(-20);
  });

  it("calls onDragEnd on pointer up", () => {
    const onDragEnd = vi.fn();
    const { container } = render(ResizeHandle, {
      props: { side: "left", onDrag: vi.fn(), onDragEnd },
    });
    const wrapper = getWrapper(container);

    firePointer(wrapper, "pointerdown", 100);
    firePointer(wrapper, "pointerup", 120);
    expect(onDragEnd).toHaveBeenCalledOnce();
  });

  it("does not call onDrag when not dragging", () => {
    const onDrag = vi.fn();
    const { container } = render(ResizeHandle, {
      props: { side: "left", onDrag },
    });
    const wrapper = getWrapper(container);

    firePointer(wrapper, "pointermove", 120);
    expect(onDrag).not.toHaveBeenCalled();
  });

  it("tracks cumulative deltas across multiple moves", () => {
    const onDrag = vi.fn();
    const { container } = render(ResizeHandle, {
      props: { side: "left", onDrag },
    });
    const wrapper = getWrapper(container);

    firePointer(wrapper, "pointerdown", 100);
    firePointer(wrapper, "pointermove", 110);
    firePointer(wrapper, "pointermove", 130);
    expect(onDrag).toHaveBeenCalledTimes(2);
    expect(onDrag).toHaveBeenNthCalledWith(1, 10);
    expect(onDrag).toHaveBeenNthCalledWith(2, 20);
  });

  it("calls onReset on double-click", () => {
    const onReset = vi.fn();
    const { container } = render(ResizeHandle, {
      props: { side: "left", onDrag: vi.fn(), onReset },
    });
    const wrapper = getWrapper(container);
    wrapper.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    expect(onReset).toHaveBeenCalledOnce();
  });

  it("stops calling onDrag after pointer up", () => {
    const onDrag = vi.fn();
    const { container } = render(ResizeHandle, {
      props: { side: "left", onDrag },
    });
    const wrapper = getWrapper(container);

    firePointer(wrapper, "pointerdown", 100);
    firePointer(wrapper, "pointermove", 110);
    firePointer(wrapper, "pointerup", 110);
    onDrag.mockClear();

    firePointer(wrapper, "pointermove", 130);
    expect(onDrag).not.toHaveBeenCalled();
  });

  it("shows active styling while dragging", async () => {
    const { container } = render(ResizeHandle, {
      props: { side: "left", onDrag: vi.fn() },
    });
    const wrapper = getWrapper(container);
    const hitArea = getHitArea(container);

    expect(hitArea.classList.contains("handle-idle")).toBe(true);
    expect(hitArea.classList.contains("handle-active")).toBe(false);

    firePointer(wrapper, "pointerdown", 100);
    await vi.waitFor(() => {
      expect(hitArea.classList.contains("handle-active")).toBe(true);
      expect(hitArea.classList.contains("handle-idle")).toBe(false);
    });

    firePointer(wrapper, "pointerup", 100);
    await vi.waitFor(() => {
      expect(hitArea.classList.contains("handle-idle")).toBe(true);
      expect(hitArea.classList.contains("handle-active")).toBe(false);
    });
  });
});
