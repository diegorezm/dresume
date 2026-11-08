<script lang="ts">
  import { t } from "./i18n.svelte";
  import type { Diagnostic } from "./api";

  interface Props {
    svg: string[];
    diagnostics: Diagnostic[];
    busy?: boolean;
    ondiagnostic?: (d: Diagnostic) => void;
  }

  let { svg, diagnostics, busy = false, ondiagnostic }: Props = $props();
  let zoom = $state(1);

  const errors = $derived(diagnostics.filter((d) => d.severity === "error"));
  const warnings = $derived(diagnostics.filter((d) => d.severity === "warning"));
</script>

<div class="preview">
  <div class="toolbar">
    <span class="status" class:err={errors.length > 0}>
      {#if busy}{t("preview.compiling")}{:else if errors.length}{t("preview.errors", { n: errors.length })}{:else}{t("preview.upToDate")}{/if}
      {#if warnings.length}· {t("preview.warnings", { n: warnings.length })}{/if}
    </span>
    <div class="zoom">
      <button class="ghost" onclick={() => (zoom = Math.max(0.4, zoom - 0.1))}>−</button>
      <span>{Math.round(zoom * 100)}%</span>
      <button class="ghost" onclick={() => (zoom = Math.min(2.5, zoom + 0.1))}>+</button>
    </div>
  </div>

  {#if diagnostics.length}
    <ul class="diagnostics">
      {#each diagnostics as d, i (i)}
        <li class={d.severity}>
          <button class="ghost" onclick={() => ondiagnostic?.(d)} disabled={!ondiagnostic || !d.file}>
            <strong>{t(d.severity === "error" ? "preview.error" : "preview.warning")}</strong>
            {#if d.file}<code>{d.file}{d.line ? `:${d.line}:${d.column}` : ""}</code>{/if}
            {d.message}
            {#each d.hints as h (h)}<div class="hint">{t("preview.hint")}: {h}</div>{/each}
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="pages" style:--zoom={zoom}>
    {#each svg as page, i (i)}
      <div class="page">{@html page}</div>
    {/each}
  </div>
</div>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--canvas);
  }
  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.4rem 0.75rem;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    color: var(--muted);
    font-size: 0.85rem;
  }
  .status.err {
    color: var(--danger);
  }
  .zoom {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
  .diagnostics {
    list-style: none;
    margin: 0;
    padding: 0.25rem 0.75rem;
    max-height: 30%;
    overflow: auto;
    font-size: 0.8rem;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }
  .diagnostics button {
    text-align: left;
    width: 100%;
    display: block;
  }
  .diagnostics .error strong {
    color: var(--danger);
  }
  .diagnostics .warning strong {
    color: var(--warn);
  }
  .hint {
    color: var(--muted);
    padding-left: 1rem;
  }
  .pages {
    flex: 1;
    overflow: auto;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
  }
  .page {
    /* Fit the pane at 100%, grow past it when zoomed in. */
    width: calc(min(595px, 100%) * var(--zoom));
    flex-shrink: 0;
    background: white;
    border-radius: 4px;
    overflow: hidden;
    box-shadow: var(--shadow), 0 0 0 1px var(--border);
  }
  .page :global(svg) {
    display: block;
    width: 100%;
    height: auto;
  }
</style>
