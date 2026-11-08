<script lang="ts">
  import { t } from "../i18n.svelte";
  // Top level of the content editor: one collapsible card per resume section.
  import Field from "./Field.svelte";
  import type { Json, Schema } from "../api";
  import { emptyFor, humanize, typeOf, type Obj } from "./util";

  interface Props {
    schema: Schema;
    value: Obj;
    onchange: (value: Obj) => void;
  }

  let { schema, value, onchange }: Props = $props();

  type Section = { key: string; schema: Schema };

  const fields = $derived(Object.entries(schema.properties ?? {}).filter(([k]) => k !== "template"));
  // Plain fields (role, summary, language…) are grouped into one "Basics" card.
  const basics = $derived(fields.filter(([, s]) => typeOf(s) !== "array" && typeOf(s) !== "object"));
  const sections = $derived<Section[]>(
    fields.filter(([, s]) => typeOf(s) === "array" || typeOf(s) === "object").map(([key, s]) => ({ key, schema: s })),
  );
  const required = $derived(new Set(schema.required ?? []));
  const present = $derived(sections.filter((s) => value[s.key] !== undefined || required.has(s.key)));
  const missing = $derived(sections.filter((s) => value[s.key] === undefined && !required.has(s.key)));

  let collapsed = $state<Set<string>>(new Set());

  function toggle(key: string) {
    const next = new Set(collapsed);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    collapsed = next;
  }

  function set(key: string, v: Json | undefined) {
    const next = { ...value };
    if (v === undefined || v === "") delete next[key];
    else next[key] = v;
    onchange(next);
  }

  function addSection(s: Section) {
    // Start list sections with one item to fill in.
    set(s.key, typeOf(s.schema) === "array" && s.schema.items ? [emptyFor(s.schema.items)] : emptyFor(s.schema));
    collapsed = new Set([...collapsed].filter((k) => k !== s.key));
  }

  function removeSection(s: Section) {
    if (!confirm(t("form.confirmRemoveSection", { name: s.schema.title ?? humanize(s.key) }))) return;
    set(s.key, undefined);
  }

  function count(key: string) {
    const v = value[key];
    return Array.isArray(v) ? v.length : null;
  }
</script>

<div class="sections">
  {#if basics.length}
    <section class="section" class:collapsed={collapsed.has("@basics")}>
      <button type="button" class="section-head" onclick={() => toggle("@basics")}>
        <span class="section-title">{t("form.basics")}</span>
        <span class="chev" aria-hidden="true">›</span>
      </button>
      {#if !collapsed.has("@basics")}
        <div class="section-body basics">
          {#each basics as [key, s] (key)}
            <div class:wide={s.format === "textarea"}>
              <Field schema={s} value={value[key]} name={key} onchange={(v) => set(key, v)} />
            </div>
          {/each}
        </div>
      {/if}
    </section>
  {/if}

  {#each present as s (s.key)}
    {@const n = count(s.key)}
    <section class="section" class:collapsed={collapsed.has(s.key)}>
      <div class="section-head-row">
        <button type="button" class="section-head" onclick={() => toggle(s.key)}>
          <span class="section-title">{s.schema.title ?? humanize(s.key)}</span>
          {#if n !== null}<span class="count">{n}</span>{/if}
          <span class="chev" aria-hidden="true">›</span>
        </button>
        {#if !required.has(s.key)}
          <button type="button" class="ghost small remove-section" title={t("form.removeSection")} onclick={() => removeSection(s)}>✕</button>
        {/if}
      </div>
      {#if !collapsed.has(s.key)}
        <div class="section-body">
          <Field schema={s.schema} value={value[s.key]} name={s.key} bare onchange={(v) => set(s.key, v ?? emptyFor(s.schema))} />
        </div>
      {/if}
    </section>
  {/each}

  {#if missing.length}
    <div class="add-sections">
      <span>{t("form.addSection")}</span>
      <div class="chips">
        {#each missing as s (s.key)}
          <button type="button" onclick={() => addSection(s)}>＋ {s.schema.title ?? humanize(s.key)}</button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .sections {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .section {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
  }
  .section-head-row {
    display: flex;
    align-items: center;
    padding-right: 0.5rem;
  }
  .section-head {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    border: none;
    background: transparent;
    padding: 0.85rem 1.1rem;
    border-radius: var(--radius);
    text-align: left;
  }
  .section-head:hover:not(:disabled) {
    background: transparent;
  }
  .section-title {
    font-weight: 700;
    font-size: 1.02rem;
  }
  .count {
    font-size: 0.75rem;
    color: var(--muted);
    background: var(--surface-2);
    border-radius: 999px;
    padding: 0 0.5rem;
  }
  .chev {
    margin-left: auto;
    color: var(--muted);
    rotate: 90deg;
    transition: rotate 0.15s;
  }
  .collapsed .chev {
    rotate: 0deg;
  }
  .remove-section {
    opacity: 0;
    font-size: 0.75rem;
  }
  .section:hover .remove-section {
    opacity: 1;
  }
  .section-body {
    padding: 0 1.1rem 1.1rem;
  }
  .basics {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }
  .basics .wide {
    grid-column: 1 / -1;
  }
  .add-sections {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem 0.25rem;
  }
  .add-sections > span {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--muted);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }
  .chips button {
    border-style: dashed;
    background: transparent;
    color: var(--muted);
    font-size: 0.9rem;
  }
  .chips button:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  @container (max-width: 520px) {
    .basics {
      grid-template-columns: 1fr;
    }
  }
</style>
