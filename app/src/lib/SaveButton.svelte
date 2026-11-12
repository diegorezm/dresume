<script lang="ts">
  import { t } from "./i18n.svelte";

  let { onclick, disabled = false }: { onclick: () => void; disabled?: boolean } = $props();

  const key = /Mac|iPhone|iPad/.test(navigator.platform) ? "⌘S" : "Ctrl+S";
</script>

<button class="primary save" {onclick} {disabled} title={t("common.saveShortcut", { key })} aria-label={t("common.save")}>
  <!-- Floppy disk -->
  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
    <path d="M17 21v-8H7v8" />
    <path d="M7 3v5h8" />
  </svg>
</button>

<style>
  .save {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding-inline: 0.75rem;
  }
</style>
