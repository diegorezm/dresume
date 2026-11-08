<script lang="ts">
  // Segmented control for a SplitPane: editor only / both / preview only.
  import { t } from "./i18n.svelte";
  import { layout, type PaneMode } from "./layout.svelte";

  interface Props {
    /** The SplitPane id this controls. */
    id: string;
    /** When the panes can't sit side by side, "split" is unavailable. */
    narrow?: boolean;
  }

  let { id, narrow = false }: Props = $props();

  const current = $derived.by(() => {
    const m = layout.mode(id);
    return narrow && m === "split" ? "left" : m;
  });

  const options: { mode: PaneMode; label: "view.editor" | "view.split" | "view.preview"; d: string }[] = [
    { mode: "left", label: "view.editor", d: "M2 2.5h12v11H2z M5 6h6 M5 8.5h6 M5 11h4" },
    { mode: "split", label: "view.split", d: "M2 2.5h12v11H2z M8 2.5v11" },
    { mode: "right", label: "view.preview", d: "M4 1.5h8v13H4z M6 5h4 M6 7.5h4 M6 10h3" },
  ];
</script>

<div class="switch" role="radiogroup" aria-label={t("view.label")}>
  {#each options as o (o.mode)}
    {#if !(narrow && o.mode === "split")}
      <button
        type="button"
        role="radio"
        aria-checked={current === o.mode}
        class:active={current === o.mode}
        title={t(o.label)}
        aria-label={t(o.label)}
        onclick={() => layout.setMode(id, o.mode)}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d={o.d} /></svg>
      </button>
    {/if}
  {/each}
</div>

<style>
  .switch {
    display: inline-flex;
    background: var(--surface-2);
    border-radius: 999px;
    padding: 2px;
    gap: 1px;
    flex-shrink: 0;
  }
  button {
    border: none;
    background: transparent;
    border-radius: 999px;
    padding: 0.25rem 0.5rem;
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  button:hover:not(:disabled) {
    background: transparent;
    color: var(--text);
  }
  button.active {
    background: var(--surface);
    color: var(--accent);
    box-shadow: var(--shadow);
  }
  svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.3;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
</style>
