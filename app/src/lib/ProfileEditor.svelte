<script lang="ts">
  import { t } from "./i18n.svelte";
  import { api } from "./api";
  import CodeEditor from "./CodeEditor.svelte";

  let text = $state("");
  let saved = $state("");
  let message = $state("");

  $effect(() => {
    api.getProfile().then((t) => (text = saved = t));
  });

  async function persist() {
    try {
      await api.saveProfile(text);
      saved = text;
      message = t("profile.saved");
    } catch (e) {
      message = String(e);
    }
    setTimeout(() => (message = ""), 3000);
  }
</script>

<section class="profile">
  <header class="topbar">
    <div class="crumbs">
      <span class="chip">☺ {t("profile.title")}{text !== saved ? " •" : ""}</span>
      <span class="chip muted">{t("profile.shared")}</span>
    </div>
    <div class="actions">
      {#if message}<span class="flash">{message}</span>{/if}
      <button class="primary" onclick={persist} disabled={text === saved}>{t("common.save")}</button>
    </div>
  </header>
  <div class="code">
    <CodeEditor name="profile.yaml" value={text} onchange={(t) => (text = t)} />
  </div>
</section>

<style>
  .profile {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .code {
    flex: 1;
    min-height: 0;
  }
</style>
