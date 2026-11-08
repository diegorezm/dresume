<script lang="ts">
  import { t } from "../i18n.svelte";
  interface Props {
    value: string[];
    placeholder?: string;
    onchange: (value: string[]) => void;
  }

  let { value, placeholder = t("form.tagPlaceholder"), onchange }: Props = $props();
  let draft = $state("");
  let input: HTMLInputElement;

  function commit() {
    const parts = draft.split(",").map((x) => x.trim()).filter(Boolean);
    if (parts.length) onchange([...value, ...parts.filter((p) => !value.includes(p))]);
    draft = "";
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      commit();
    } else if (e.key === "Backspace" && draft === "" && value.length) {
      draft = value[value.length - 1];
      onchange(value.slice(0, -1));
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="tags" onclick={() => input.focus()}>
  {#each value as tag, i (tag + i)}
    <span class="tag">
      {tag}
      <button type="button" aria-label={t("form.removeTag", { tag })} onclick={() => onchange(value.filter((_, k) => k !== i))}>✕</button>
    </span>
  {/each}
  <input bind:this={input} bind:value={draft} {onkeydown} onblur={commit} placeholder={value.length ? "" : placeholder} />
</div>

<style>
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    padding: 0.4rem;
    background: var(--surface-2);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: text;
  }
  .tags:focus-within {
    border-color: var(--accent);
    background: var(--surface);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.15rem 0.3rem 0.15rem 0.65rem;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--text);
    font-size: 0.9rem;
  }
  .tag button {
    border: none;
    background: transparent;
    padding: 0 0.25rem;
    font-size: 0.65rem;
    color: var(--muted);
    border-radius: 50%;
  }
  .tag button:hover {
    color: var(--danger);
    background: transparent;
  }
  input {
    flex: 1;
    min-width: 8rem;
    border: none;
    background: transparent;
    padding: 0.2rem 0.3rem;
  }
  input:focus {
    box-shadow: none;
    background: transparent;
  }
</style>
