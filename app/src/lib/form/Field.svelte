<script lang="ts">
  // Renders one schema value: text, textarea, select, number, bool, string lists, item lists, objects.
  import Field from "./Field.svelte";
  import ItemList from "./ItemList.svelte";
  import TagInput from "./TagInput.svelte";
  import BulletList from "./BulletList.svelte";
  import type { Json, Schema } from "../api";
  import { humanize, stringListStyle, typeOf, type Obj } from "./util";

  interface Props {
    schema: Schema;
    value: Json | undefined;
    /** Property name, used for labels. */
    name: string;
    /** Hide the label (when a parent already shows it). */
    bare?: boolean;
    onchange: (value: Json | undefined) => void;
  }

  let { schema, value, name, bare = false, onchange }: Props = $props();

  const kind = $derived(typeOf(schema));
  const label = $derived(schema.title ?? humanize(name));
  const itemKind = $derived(schema.items ? typeOf(schema.items) : undefined);

  function setKey(key: string, v: Json | undefined) {
    const obj = { ...((value as Obj) ?? {}) };
    if (v === undefined || v === "") delete obj[key];
    else obj[key] = v;
    onchange(obj);
  }

  /** Grows the textarea with its content. */
  function autosize(el: HTMLTextAreaElement) {
    const fit = () => {
      el.style.height = "auto";
      el.style.height = `${el.scrollHeight + 2}px`;
    };
    fit();
    el.addEventListener("input", fit);
  }
</script>

{#snippet head()}
  {#if !bare}<span class="label">{label}</span>{/if}
{/snippet}

{#if kind === "object"}
  {@const obj = (value as Obj) ?? {}}
  {@const entries = Object.entries(schema.properties ?? {}).filter(([k]) => k !== "template")}
  <div class="object">
    {#each entries as [key, sub] (key)}
      {@const wide = typeOf(sub) === "array" || sub.format === "textarea"}
      <div class="cell" class:wide>
        <Field schema={sub} value={obj[key]} name={key} onchange={(v) => setKey(key, v)} />
      </div>
    {/each}
  </div>
{:else if kind === "array" && itemKind === "object"}
  <div class="field">
    {@render head()}
    <ItemList {schema} {name} value={(value as Json[]) ?? []} {onchange} />
  </div>
{:else if kind === "array"}
  <div class="field">
    {@render head()}
    {#if stringListStyle(schema) === "tags"}
      <TagInput value={(value as string[]) ?? []} {onchange} />
    {:else}
      <BulletList value={(value as string[]) ?? []} {onchange} />
    {/if}
  </div>
{:else if kind === "boolean"}
  <label class="check">
    <input type="checkbox" checked={!!value} onchange={(e) => onchange(e.currentTarget.checked)} />
    {label}
  </label>
{:else if kind === "number" || kind === "integer"}
  <label class="field">
    {@render head()}
    <input type="number" value={value ?? ""} oninput={(e) => onchange(e.currentTarget.valueAsNumber)} />
  </label>
{:else if schema.enum}
  <label class="field">
    {@render head()}
    <select value={value ?? ""} onchange={(e) => onchange(e.currentTarget.value || undefined)}>
      <option value="">—</option>
      {#each schema.enum as opt (opt)}<option value={opt}>{opt}</option>{/each}
    </select>
  </label>
{:else if schema.format === "textarea"}
  <label class="field">
    {@render head()}
    <textarea rows="3" use:autosize value={(value as string) ?? ""} oninput={(e) => onchange(e.currentTarget.value)}></textarea>
  </label>
{:else}
  <label class="field">
    {@render head()}
    <input
      type="text"
      value={value ?? ""}
      placeholder={schema.format === "uri" ? "https://…" : ""}
      oninput={(e) => onchange(e.currentTarget.value)}
    />
  </label>
{/if}


<style>
  .object {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }
  .cell.wide {
    grid-column: 1 / -1;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .label {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--muted);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .check input {
    width: auto;
  }
  textarea {
    resize: none;
    overflow: hidden;
  }
  @container (max-width: 520px) {
    .object {
      grid-template-columns: 1fr;
    }
  }
</style>
