<script lang="ts">
  import { t } from "./i18n.svelte";

  export interface MenuItem {
    label: string;
    onclick: () => void;
    danger?: boolean;
  }

  let { items }: { items: MenuItem[] } = $props();
  let open = $state(false);
  let root: HTMLDivElement;

  function pick(item: MenuItem) {
    open = false;
    item.onclick();
  }

  function outside(e: MouseEvent) {
    if (open && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onclick={outside} onkeydown={(e) => open && e.key === "Escape" && (open = false)} />

<div class="more" bind:this={root}>
  <button class="ghost" class:active={open} onclick={() => (open = !open)} title={t("common.more")} aria-haspopup="menu" aria-expanded={open}>⋯</button>
  {#if open}
    <div class="menu" role="menu">
      {#each items as item (item.label)}
        <button role="menuitem" class:danger={item.danger} onclick={() => pick(item)}>{item.label}</button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .more {
    position: relative;
  }
  .more > button {
    font-size: 1.1rem;
    line-height: 1;
  }
  .more > button.active {
    background: var(--surface-2);
  }
  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 0.35rem);
    z-index: 20;
    min-width: 10rem;
    padding: 0.3rem;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow);
  }
  .menu button {
    border: none;
    background: none;
    border-radius: calc(var(--radius-sm) - 2px);
    text-align: left;
    padding: 0.45rem 0.7rem;
  }
  .menu button:hover,
  .menu button:focus-visible {
    background: var(--surface-2);
  }
  .menu button.danger {
    color: var(--danger);
  }
</style>
