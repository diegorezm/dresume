<script lang="ts">
  import { t } from "./i18n.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { appearance } from "./theme.svelte";
  import Logo from "./Logo.svelte";

  const win = getCurrentWindow();
  let maximized = $state(false);

  $effect(() => {
    win.isMaximized().then((m) => (maximized = m));
    const unlisten = win.onResized(async () => (maximized = await win.isMaximized()));
    return () => {
      unlisten.then((f) => f());
    };
  });

  // Keep native bits (context menus, scrollbars, the taskbar preview) in sync with the theme.
  $effect(() => {
    win.setTheme(appearance.dark ? "dark" : "light").catch(() => {});
  });
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <span class="logo"><Logo size={18} /></span>
    <span data-tauri-drag-region>dresume</span>
  </div>
  <div class="controls">
    <button aria-label={t("window.minimize")} onclick={() => win.minimize()}>
      <svg viewBox="0 0 10 10"><path d="M0 5h10" /></svg>
    </button>
    <button aria-label={maximized ? t("window.restore") : t("window.maximize")} onclick={() => win.toggleMaximize()}>
      {#if maximized}
        <svg viewBox="0 0 10 10"><path d="M2 0h8v8M0 2h8v8H0z" /></svg>
      {:else}
        <svg viewBox="0 0 10 10"><path d="M0 0h10v10H0z" /></svg>
      {/if}
    </button>
    <button class="close" aria-label={t("common.close")} onclick={() => win.close()}>
      <svg viewBox="0 0 10 10"><path d="M0 0l10 10M10 0L0 10" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    height: 34px;
    flex-shrink: 0;
    display: flex;
    justify-content: space-between;
    align-items: stretch;
    background: var(--sidebar);
    border-bottom: 1px solid var(--border);
    user-select: none;
    -webkit-user-select: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding-left: 0.75rem;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--muted);
  }
  .logo {
    display: grid;
    pointer-events: none;
  }
  .controls {
    display: flex;
  }
  .controls button {
    width: 46px;
    border: none;
    border-radius: 0;
    background: transparent;
    padding: 0;
    display: grid;
    place-items: center;
    color: var(--text);
  }
  .controls button:hover {
    background: var(--surface-2);
  }
  .controls .close:hover {
    background: #e81123;
    color: #fff;
  }
  svg {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1;
  }
</style>
