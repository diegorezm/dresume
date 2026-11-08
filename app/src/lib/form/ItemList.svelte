<script lang="ts">
  import { t } from "../i18n.svelte";
  // A list of object items (experience, education…) as collapsible, draggable cards.
  import Field from "./Field.svelte";
  import type { Json, Schema } from "../api";
  import { emptyFor, itemSummary, singular } from "./util";

  interface Props {
    schema: Schema;
    name: string;
    value: Json[];
    onchange: (value: Json[]) => void;
  }

  let { schema, name, value, onchange }: Props = $props();

  const itemSchema = $derived(schema.items ?? {});
  const noun = $derived(singular(schema, name));

  /** Open cards, by index. Starts collapsed when there are many items. */
  // svelte-ignore state_referenced_locally
  let open = $state<Set<number>>(new Set(value.length <= 1 ? [0] : []));
  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);

  function toggle(i: number) {
    const next = new Set(open);
    if (next.has(i)) next.delete(i);
    else next.add(i);
    open = next;
  }

  /** Re-maps open indices after items move. `map(oldIndex) -> newIndex | null`. */
  function remapOpen(map: (i: number) => number | null) {
    open = new Set([...open].map(map).filter((i): i is number => i !== null));
  }

  function setItem(i: number, v: Json | undefined) {
    onchange(value.map((x, k) => (k === i ? (v ?? emptyFor(itemSchema)) : x)));
  }

  function add() {
    onchange([...value, emptyFor(itemSchema)]);
    open = new Set([...open, value.length]);
  }

  function duplicate(i: number) {
    onchange([...value.slice(0, i + 1), JSON.parse(JSON.stringify(value[i])) as Json, ...value.slice(i + 1)]);
    remapOpen((k) => (k > i ? k + 1 : k));
    open = new Set([...open, i + 1]);
  }

  function remove(i: number) {
    const { title } = itemSummary(value[i], itemSchema);
    if (title && !confirm(t("form.confirmRemoveItem", { title }))) return;
    onchange(value.filter((_, k) => k !== i));
    remapOpen((k) => (k === i ? null : k > i ? k - 1 : k));
  }

  function move(from: number, to: number) {
    if (from === to) return;
    const arr = [...value];
    const [item] = arr.splice(from, 1);
    arr.splice(to, 0, item);
    onchange(arr);
    remapOpen((k) => {
      if (k === from) return to;
      if (from < to && k > from && k <= to) return k - 1;
      if (from > to && k >= to && k < from) return k + 1;
      return k;
    });
  }
</script>

<div class="items">
  {#each value as item, i (i)}
    {@const summary = itemSummary(item, itemSchema)}
    <div
      class="item"
      class:open={open.has(i)}
      class:dragging={dragFrom === i}
      class:over={dragOver === i && dragFrom !== i}
      role="listitem"
      ondragover={(e) => { if (dragFrom !== null) { e.preventDefault(); dragOver = i; } }}
      ondrop={(e) => { e.preventDefault(); if (dragFrom !== null) move(dragFrom, i); dragFrom = dragOver = null; }}
    >
      <div class="item-head">
        <span
          class="grip"
          draggable="true"
          role="button"
          tabindex="-1"
          aria-label={t("form.dragToReorder")}
          title={t("form.dragToReorder")}
          ondragstart={(e) => { dragFrom = i; e.dataTransfer?.setData("text/plain", String(i)); }}
          ondragend={() => (dragFrom = dragOver = null)}
        >⋮⋮</span>
        <button type="button" class="summary" onclick={() => toggle(i)} aria-expanded={open.has(i)}>
          <span class="title" class:placeholder={!summary.title}>{summary.title || t("form.newItem", { noun })}</span>
          {#if summary.meta}<span class="meta">{summary.meta}</span>{/if}
          <span class="chev" aria-hidden="true">›</span>
        </button>
        <div class="tools">
          <button type="button" class="ghost" title={t("form.duplicate")} onclick={() => duplicate(i)}>⧉</button>
          <button type="button" class="ghost danger" title={t("form.remove")} onclick={() => remove(i)}>✕</button>
        </div>
      </div>
      {#if open.has(i)}
        <div class="item-body">
          <Field schema={itemSchema} value={item} name={noun} bare onchange={(v) => setItem(i, v)} />
        </div>
      {/if}
    </div>
  {/each}

  <button type="button" class="add" onclick={add}>{t("form.addItem", { noun })}</button>
</div>

<style>
  .items {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .item {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
    transition: border-color 0.15s, box-shadow 0.15s, opacity 0.15s;
  }
  .item.open {
    background: var(--surface);
    box-shadow: var(--shadow);
  }
  .item.dragging {
    opacity: 0.45;
  }
  .item.over {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .item-head {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2rem 0.3rem 0.2rem 0.2rem;
  }
  .grip {
    cursor: grab;
    color: var(--muted);
    opacity: 0.5;
    padding: 0.3rem 0.25rem;
    letter-spacing: -0.2em;
    user-select: none;
    font-size: 0.85rem;
  }
  .item:hover .grip {
    opacity: 1;
  }
  .summary {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    border: none;
    background: transparent;
    padding: 0.45rem 0.4rem;
    text-align: left;
    border-radius: var(--radius-sm);
  }
  .summary:hover:not(:disabled) {
    background: transparent;
  }
  .title {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title.placeholder {
    color: var(--muted);
    font-style: italic;
    font-weight: 400;
  }
  .meta {
    color: var(--muted);
    font-size: 0.85rem;
    white-space: nowrap;
  }
  .chev {
    color: var(--muted);
    transition: rotate 0.15s;
  }
  .item.open .chev {
    rotate: 90deg;
  }
  .tools {
    display: flex;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .item:hover .tools,
  .item:focus-within .tools {
    opacity: 1;
  }
  .tools button {
    font-size: 0.8rem;
  }
  .item-body {
    padding: 0.4rem 0.9rem 0.9rem;
    border-top: 1px solid var(--border);
    padding-top: 0.85rem;
  }
  .add {
    align-self: flex-start;
    border-style: dashed;
    color: var(--muted);
    background: transparent;
  }
  .add:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-soft);
  }
</style>
