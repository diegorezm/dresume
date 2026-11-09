<script lang="ts">
  import "../app.css";
  import { isTauri } from "@tauri-apps/api/core";
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
</script>

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
