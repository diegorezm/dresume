<script lang="ts">
  import "../app.css";
  import { isTauri } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { appearance } from "$lib/theme.svelte";
  import TitleBar from "$lib/TitleBar.svelte";
  import UpdateBanner from "$lib/UpdateBanner.svelte";
  import { updater } from "$lib/updater.svelte";

  let { children } = $props();

  // Apply defaults immediately, then the saved settings once loaded.
  appearance.apply();
  const ready = (async () => {
    // Outside Tauri (`pnpm dev` in a browser), fake the backend with fixture data.
    if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
      await (await import("$lib/dev/mock")).installMock();
    }
    await appearance.load();
  })();
  ready.then(() => updater.check());

  /** Every link (preview SVG, chat, anywhere) opens in the system browser, never in the app window. */
  function onclick(e: MouseEvent) {
    const a = (e.target as Element | null)?.closest?.("a");
    // SVG links (Typst preview) keep the URL in href or xlink:href.
    const href = a?.getAttribute("href") ?? a?.getAttribute("xlink:href");
    if (!a || !href || href.startsWith("#")) return;
    e.preventDefault();
    if (isTauri() && /^(https?|mailto|tel):/i.test(href)) openUrl(href);
    else if (!isTauri()) window.open(href, "_blank", "noopener");
  }
</script>

<svelte:document {onclick} onauxclick={onclick} />

<div class="window">
  {#if isTauri()}<TitleBar />{/if}
  <div class="content">
    {#await ready then}
      {@render children()}
      <UpdateBanner />
    {/await}
  </div>
</div>

<style>
  .window {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .content {
    flex: 1;
    min-height: 0;
  }
</style>
