<script lang="ts">
  interface Props {
    side: "left" | "right";
    onDrag: (delta: number) => void;
    onDragStart?: () => void;
    onDragEnd?: () => void;
    onReset?: () => void;
  }

  let { side, onDrag, onDragStart, onDragEnd, onReset }: Props = $props();

  let dragging = $state(false);
  let startX = 0;

  function handlePointerDown(e: PointerEvent) {
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);
    startX = e.clientX;
    dragging = true;
    onDragStart?.();
  }

  function handlePointerMove(e: PointerEvent) {
    if (!dragging) return;
    const delta = e.clientX - startX;
    startX = e.clientX;
    // Left handle: moving right grows the left pane (positive delta)
    // Right handle: moving right shrinks the right pane (negative delta)
    onDrag(side === "left" ? delta : -delta);
  }

  function handlePointerUp(e: PointerEvent) {
    if (!dragging) return;
    const target = e.currentTarget as HTMLElement;
    target.releasePointerCapture(e.pointerId);
    dragging = false;
    onDragEnd?.();
  }

  function handleDoubleClick() {
    onReset?.();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="relative w-0 shrink-0"
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  ondblclick={handleDoubleClick}
>
  <div
    class="absolute top-0 bottom-0 -left-1.5 w-3 cursor-col-resize z-10
      {dragging ? 'handle-active' : 'handle-idle'}"
  ></div>
</div>

<style>
  .handle-idle::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 0;
    transition:
      width 0.15s,
      margin-left 0.15s;
  }

  .handle-idle:hover::after {
    width: 1px;
    margin-left: -0.5px;
    background: var(--color-text-faint);
  }

  .handle-active::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 2px;
    margin-left: -1px;
    background: var(--color-accent);
  }
</style>
