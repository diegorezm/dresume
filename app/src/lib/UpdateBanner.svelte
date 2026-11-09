<script lang="ts">
  import { updater } from "./updater.svelte";
  import { t } from "./i18n.svelte";
</script>

{#if !updater.dismissed && (updater.status === "available" || updater.status === "downloading")}
  <div class="update card" role="status">
    {#if updater.status === "downloading"}
      <span>{t("update.downloading", { n: Math.round(updater.progress * 100) })}</span>
    {:else}
      <span>{t("update.available", { v: updater.version })}</span>
      <button class="ghost" onclick={() => (updater.dismissed = true)}>{t("update.later")}</button>
      <button class="primary" onclick={() => updater.install()}>{t("update.install")}</button>
    {/if}
  </div>
{/if}

<style>
  .update {
    position: fixed;
    right: 1rem;
    bottom: 1rem;
    z-index: 50;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.6rem 0.6rem 0.6rem 1rem;
    box-shadow: var(--shadow);
    font-size: 0.85rem;
  }
</style>
