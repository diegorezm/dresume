<script lang="ts">
  // Two panes with a draggable divider. The split ratio is remembered per `id` in localStorage,
  // and which panes are visible comes from the shared layout store. When the container is too
  // narrow for both panes, "split" falls back to a single pane (the parent offers a switch).
  import type { Snippet } from "svelte";
  import { t } from "./i18n.svelte";
  import { layout } from "./layout.svelte";

  interface Props {
    /** Storage key for the ratio and the layout mode. */
    id: string;
    /** Initial left width, as a fraction of the container. */
    initial?: number;
    /** Minimum width of each pane, in px. */
    min?: number;
    /** Set to true while the container can't fit both panes. */
    narrow?: boolean;
    left: Snippet;
    right: Snippet;
  }

  let { id, initial = 0.5, min = 320, narrow = $bindable(false), left, right }: Props = $props();

  const key = $derived(`dresume.split.${id}`);
  let ratio = $state(read());
  let container: HTMLDivElement;
  let dragging = $state(false);

  // Both panes stay mounted (editor state, scroll) and are hidden with CSS.
  const mode = $derived.by(() => {
    const m = layout.mode(id);
    return narrow && m === "split" ? "left" : m;
  });

  $effect(() => {
    const ro = new ResizeObserver(([entry]) => {
      narrow = entry.contentRect.width < min * 2;
    });
    ro.observe(container);
    return () => ro.disconnect();
  });

  function read(): number {
    try {
      const v = Number(localStorage.getItem(`dresume.split.${id}`));
      if (v > 0 && v < 1) return v;
    } catch {}
    return initial;
  }

  function save() {
    try {
      localStorage.setItem(key, String(ratio));
    } catch {}
  }

  function clamp(r: number) {
    const width = container?.clientWidth ?? 0;
    if (!width) return r;
    const lo = Math.min(min / width, 0.5);
    return Math.min(1 - lo, Math.max(lo, r));
  }

  function onpointerdown(e: PointerEvent) {
    e.preventDefault();
    dragging = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onpointermove(e: PointerEvent) {
    if (!dragging) return;
    const rect = container.getBoundingClientRect();
    ratio = clamp((e.clientX - rect.left) / rect.width);
  }

  function onpointerup() {
    if (!dragging) return;
    dragging = false;
    save();
  }

  function onkeydown(e: KeyboardEvent) {
    const step = e.shiftKey ? 0.1 : 0.02;
    if (e.key === "ArrowLeft") ratio = clamp(ratio - step);
    else if (e.key === "ArrowRight") ratio = clamp(ratio + step);
    else return;
    e.preventDefault();
    save();
  }
</script>

<div
  class="split mode-{mode}"
  class:dragging
  bind:this={container}
  style:--left="{ratio * 100}%"
>
  <div class="pane left">{@render left()}</div>
  <!-- A focusable separator is an interactive ARIA widget (resizable with arrow keys). -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="divider"
    role="separator"
    aria-orientation="vertical"
    aria-label={t("common.resize")}
    aria-valuenow={Math.round(ratio * 100)}
    tabindex="0"
    {onpointerdown}
    {onpointermove}
    {onpointerup}
    onpointercancel={onpointerup}
    ondblclick={() => {
      ratio = initial;
      save();
    }}
    {onkeydown}
  ></div>
  <div class="pane right">{@render right()}</div>
</div>

<style>
  .split {
    display: grid;
    grid-template-columns: var(--left) 0 1fr;
    height: 100%;
    min-height: 0;
  }
  .split.mode-left,
  .split.mode-right {
    grid-template-columns: 1fr;
  }
  .mode-left .right,
  .mode-right .left,
  .mode-left .divider,
  .mode-right .divider {
    display: none;
  }
  .split.dragging {
    cursor: col-resize;
    user-select: none;
  }
  /* The preview's SVG would swallow pointer events while dragging. */
  .split.dragging .pane {
    pointer-events: none;
  }
  .pane {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .divider {
    position: relative;
    z-index: 2;
    width: 0;
    cursor: col-resize;
    outline: none;
  }
  /* Wide invisible hit area with a thin visible line in the middle. */
  .divider::before {
    content: "";
    position: absolute;
    inset: 0 -5px;
  }
  .divider::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: -0.5px;
    width: 1px;
    background: var(--border);
    transition: background 0.15s, width 0.15s, left 0.15s;
  }
  .divider:hover::after,
  .divider:focus-visible::after,
  .dragging .divider::after {
    left: -1.5px;
    width: 3px;
    background: var(--accent);
  }
</style>
