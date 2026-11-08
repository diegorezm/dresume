<script lang="ts">
  import { t } from "../i18n.svelte";
  import { tick } from "svelte";

  interface Props {
    value: string[];
    placeholder?: string;
    onchange: (value: string[]) => void;
  }

  let { value, placeholder = t("form.bulletPlaceholder"), onchange }: Props = $props();
  let list: HTMLElement;

  async function focusAt(i: number, atEnd = true) {
    await tick();
    const el = list.querySelectorAll<HTMLTextAreaElement>("textarea")[i];
    if (!el) return;
    el.focus();
    const pos = atEnd ? el.value.length : 0;
    el.setSelectionRange(pos, pos);
  }

  function set(i: number, text: string) {
    onchange(value.map((v, k) => (k === i ? text : v)));
  }

  function onkeydown(e: KeyboardEvent, i: number) {
    const el = e.currentTarget as HTMLTextAreaElement;
    if (e.key === "Enter" && !e.shiftKey) {
      // Split at the cursor into a new bullet.
      e.preventDefault();
      const [before, after] = [el.value.slice(0, el.selectionStart), el.value.slice(el.selectionEnd)];
      onchange([...value.slice(0, i), before, after, ...value.slice(i + 1)]);
      focusAt(i + 1, false);
    } else if (e.key === "Backspace" && el.selectionStart === 0 && el.selectionEnd === 0 && i > 0) {
      // Merge into the previous bullet.
      e.preventDefault();
      const prev = value[i - 1];
      onchange([...value.slice(0, i - 1), prev + el.value, ...value.slice(i + 1)]);
      tick().then(() => {
        const t = list.querySelectorAll<HTMLTextAreaElement>("textarea")[i - 1];
        t?.focus();
        t?.setSelectionRange(prev.length, prev.length);
      });
    } else if (e.key === "ArrowUp" && el.selectionStart === 0 && i > 0) {
      e.preventDefault();
      focusAt(i - 1);
    } else if (e.key === "ArrowDown" && el.selectionEnd === el.value.length && i < value.length - 1) {
      e.preventDefault();
      focusAt(i + 1, false);
    }
  }

  /** Grows the textarea with its content; re-fits when the text changes from outside. */
  function autosize(el: HTMLTextAreaElement, _text: string) {
    const fit = () => {
      el.style.height = "auto";
      el.style.height = `${el.scrollHeight}px`;
    };
    fit();
    el.addEventListener("input", fit);
    return { update: fit };
  }
</script>

<ul class="bullets" bind:this={list}>
  {#each value as item, i (i)}
    <li>
      <span class="dot" aria-hidden="true"></span>
      <textarea
        rows="1"
        value={item}
        {placeholder}
        use:autosize={item}
        oninput={(e) => set(i, e.currentTarget.value)}
        onkeydown={(e) => onkeydown(e, i)}
      ></textarea>
      <button type="button" class="ghost remove" aria-label={t("form.removeBullet")} onclick={() => onchange(value.filter((_, k) => k !== i))}>✕</button>
    </li>
  {/each}
</ul>
<button type="button" class="ghost add" onclick={() => { onchange([...value, ""]); focusAt(value.length); }}>{t("form.addBullet")}</button>

<style>
  .bullets {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  li {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    border-radius: var(--radius-sm);
    padding: 0 0.25rem;
  }
  li:hover,
  li:focus-within {
    background: var(--surface-2);
  }
  .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--accent);
    margin-top: 0.85rem;
    flex-shrink: 0;
  }
  textarea {
    flex: 1;
    resize: none;
    overflow: hidden;
    background: transparent;
    border: none;
    padding: 0.4rem 0.2rem;
    line-height: 1.45;
  }
  textarea:focus {
    box-shadow: none;
    background: transparent;
  }
  .remove {
    opacity: 0;
    font-size: 0.7rem;
    margin-top: 0.3rem;
  }
  li:hover .remove,
  li:focus-within .remove {
    opacity: 1;
  }
  .add {
    align-self: flex-start;
    margin-top: 0.15rem;
    font-size: 0.85rem;
  }
</style>
