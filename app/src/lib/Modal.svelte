<script lang="ts">
  import { t } from "./i18n.svelte";
  import type { Snippet } from "svelte";

  interface Props {
    icon: string;
    title: string;
    subtitle: string;
    submitLabel: string;
    canSubmit: boolean;
    busy?: boolean;
    error?: string;
    onsubmit: () => void;
    onclose: () => void;
    children: Snippet;
  }

  let { icon, title, subtitle, submitLabel, canSubmit, busy = false, error = "", onsubmit, onclose, children }: Props =
    $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    dialog.showModal();
  });

  export function close() {
    dialog.close();
  }
</script>

<dialog bind:this={dialog} {onclose} onclick={(e) => e.target === dialog && dialog.close()}>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      if (canSubmit && !busy) onsubmit();
    }}
  >
    <header>
      <div class="bubble">{icon}</div>
      <div>
        <h2>{title}</h2>
        <p>{subtitle}</p>
      </div>
      <button type="button" class="ghost close" aria-label={t("common.close")} onclick={() => dialog.close()}>✕</button>
    </header>

    {@render children()}

    {#if error}<p class="error">{error}</p>{/if}

    <footer>
      <button type="button" class="ghost" onclick={() => dialog.close()}>{t("common.cancel")}</button>
      <button class="primary" type="submit" disabled={!canSubmit || busy}>{busy ? t("common.creating") : submitLabel}</button>
    </footer>
  </form>
</dialog>

<style>
  dialog {
    width: min(620px, calc(100vw - 2rem));
    max-height: calc(100vh - 4rem);
    padding: 0;
    border: 1px solid var(--border);
    border-radius: calc(var(--radius) + 4px);
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 24px 80px -12px rgb(0 0 0 / 0.35), var(--shadow);
    animation: pop 0.22s cubic-bezier(0.2, 0.9, 0.3, 1.1);
  }
  dialog::backdrop {
    background: color-mix(in srgb, var(--bg) 45%, rgb(0 0 0 / 0.35));
    backdrop-filter: blur(4px);
    animation: fade 0.2s ease;
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    padding: 1.5rem;
  }
  header {
    display: flex;
    gap: 1rem;
    align-items: flex-start;
  }
  header .bubble {
    width: 44px;
    height: 44px;
    font-size: 1.25rem;
    flex-shrink: 0;
  }
  h2 {
    margin: 0.1rem 0 0.15rem;
    font-size: 1.35rem;
    letter-spacing: -0.01em;
  }
  header p {
    margin: 0;
    color: var(--muted);
    font-size: 0.9rem;
  }
  .close {
    margin-left: auto;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
  }
  .error {
    margin: 0;
    color: var(--danger);
    font-size: 0.85rem;
  }

  /* Shared pieces for dialog bodies. */
  form :global(.m-label) {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  form :global(.m-label > span),
  form :global(.m-section) {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--muted);
  }
  form :global(.m-section) {
    margin-bottom: -0.6rem;
  }
  form :global(.m-id) {
    display: flex;
    align-items: center;
    background: var(--surface-2);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding-left: 0.65rem;
  }
  form :global(.m-id:focus-within) {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  form :global(.m-id.bad) {
    border-color: var(--danger);
  }
  form :global(.m-id code) {
    color: var(--muted);
  }
  form :global(.m-id input) {
    font-family: var(--mono);
    font-size: 0.88em;
    padding-left: 0.1rem;
    background: transparent;
    border: none;
    box-shadow: none;
  }
  form :global(.m-warn) {
    color: var(--danger);
  }
  form :global(.m-options) {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 0.6rem;
    max-height: 300px;
    overflow: auto;
    padding: 2px;
  }
  form :global(.m-option) {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.6rem;
    padding: 0.6rem;
    text-align: left;
    border-radius: var(--radius);
    border: 1.5px solid var(--border);
    background: var(--bg);
    transition: border-color 0.15s, transform 0.15s, box-shadow 0.15s;
  }
  form :global(.m-option:hover:not(:disabled)) {
    background: var(--bg);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    transform: translateY(-1px);
  }
  form :global(.m-option.active) {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  form :global(.m-option-text) {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    padding: 0 0.15rem 0.1rem;
  }
  form :global(.m-option-text small) {
    color: var(--muted);
    font-size: 0.8rem;
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  form :global(.m-check) {
    position: absolute;
    top: 0.9rem;
    right: 0.9rem;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 0.7rem;
    background: var(--accent);
    color: var(--accent-text);
    animation: pop 0.18s ease;
  }
  /* Miniature resume page used as an option thumbnail. */
  form :global(.m-thumb) {
    height: 92px;
    border-radius: calc(var(--radius) - 4px);
    background: var(--surface);
    border: 1px solid var(--border);
    padding: 12px 18px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow: hidden;
  }
  form :global(.m-thumb i) {
    display: block;
    height: 3px;
    width: 80%;
    border-radius: 2px;
    background: var(--border);
  }
  form :global(.m-thumb i.t1) {
    height: 6px;
    width: 45%;
    background: var(--text);
    opacity: 0.75;
  }
  form :global(.m-thumb i.t2) {
    width: 30%;
    background: var(--muted);
    opacity: 0.5;
    margin-bottom: 3px;
  }
  form :global(.m-thumb i.r) {
    width: 25%;
    height: 2px;
    background: var(--accent);
    margin-top: 3px;
  }
  form :global(.m-thumb i.s) {
    width: 60%;
  }
  form :global(.m-thumb.accent) {
    background: color-mix(in srgb, var(--accent) 5%, var(--surface));
  }
  form :global(.m-thumb.dashed) {
    align-items: center;
    justify-content: center;
    border-style: dashed;
    background: transparent;
    font-size: 1.4rem;
    color: var(--muted);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(6px);
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    dialog,
    dialog::backdrop,
    form :global(.m-check) {
      animation: none;
    }
  }
</style>
