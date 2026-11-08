<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { appearance, type FontChoice, type Radius, type Theme } from "./theme.svelte";
  import { LOCALES, i18n, t, type Key, type Locale } from "./i18n.svelte";

  let customName = $state("");

  const fonts: { id: FontChoice; label: string }[] = $derived([
    { id: "figtree", label: "Figtree" },
    { id: "inter", label: "Inter" },
    { id: "system", label: t("settings.fontSystem") },
    { id: "serif", label: t("settings.fontSerif") },
  ]);
  const radii: { id: Radius; label: Key }[] = [
    { id: "sharp", label: "settings.sharp" },
    { id: "soft", label: "settings.soft" },
    { id: "round", label: "settings.round" },
  ];

  const s = $derived(appearance.settings);
  const accent = $derived(s.accent ?? appearance.theme.colors.accent);

  function swatches(th: Theme) {
    return [th.colors.bg, th.colors.surface, th.colors.text, th.colors.accent];
  }
</script>

<div class="page">
  <header class="page-head">
    <div class="bubble">⚙</div>
    <h1>{t("settings.title")}</h1>
    <p class="sub">{t("settings.subtitle")}</p>
  </header>

  <section class="card">
    <h3>{t("settings.language")}</h3>
    <div class="segmented">
      {#each Object.entries(LOCALES) as [code, label] (code)}
        <button class:active={i18n.locale === code} onclick={() => appearance.update({ locale: code as Locale })}>{label}</button>
      {/each}
    </div>
  </section>

  <section class="card">
    <div class="card-head">
      <h3>{t("settings.theme")}</h3>
      <button class="pill" onclick={() => invoke("open_themes_dir")}>{t("settings.openThemes")}</button>
    </div>
    <div class="themes">
      {#each appearance.themes as th (th.id)}
        <button
          class="theme"
          class:active={th.id === s.theme}
          style:background={th.colors.surface}
          style:color={th.colors.text}
          style:border-color={th.id === s.theme ? accent : th.colors.border}
          onclick={() => appearance.update({ theme: th.id, accent: null })}
        >
          <div class="swatches">
            {#each swatches(th) as color, i (i)}<span style:background={color}></span>{/each}
          </div>
          <span class="theme-name">{th.name}</span>
          <span class="theme-kind" style:color={th.colors.muted}>{th.dark ? t("settings.dark") : t("settings.light")}</span>
        </button>
      {/each}
    </div>
    <p class="hint">{t("settings.themesHint")}</p>
  </section>

  <section class="card">
    <h3>{t("settings.accent")}</h3>
    <div class="row">
      <input type="color" value={accent} oninput={(e) => appearance.update({ accent: e.currentTarget.value })} />
      <code>{accent}</code>
      {#if s.accent}
        <button class="pill" onclick={() => appearance.update({ accent: null })}>{t("settings.resetAccent")}</button>
      {/if}
    </div>
    {#if s.accent}
      <form class="row" onsubmit={(e) => { e.preventDefault(); if (customName.trim()) appearance.saveAsCustom(customName.trim()); customName = ""; }}>
        <input placeholder={t("settings.saveAsPlaceholder")} bind:value={customName} />
        <button class="primary" type="submit" disabled={!customName.trim()}>{t("settings.saveTheme")}</button>
      </form>
    {/if}
  </section>

  <section class="card grid2">
    <div>
      <h3>{t("settings.font")}</h3>
      <div class="segmented">
        {#each fonts as f (f.id)}
          <button class:active={s.font === f.id} onclick={() => appearance.update({ font: f.id })}>{f.label}</button>
        {/each}
      </div>
    </div>
    <div>
      <h3>{t("settings.corners")}</h3>
      <div class="segmented">
        {#each radii as r (r.id)}
          <button class:active={s.radius === r.id} onclick={() => appearance.update({ radius: r.id })}>{t(r.label)}</button>
        {/each}
      </div>
    </div>
    <div>
      <h3>{t("settings.size", { n: Math.round(s.scale * 100) })}</h3>
      <input
        type="range"
        min="0.85"
        max="1.2"
        step="0.05"
        value={s.scale}
        onchange={(e) => appearance.update({ scale: e.currentTarget.valueAsNumber })}
      />
    </div>
  </section>
</div>

<style>
  .page {
    max-width: 860px;
    margin: 0 auto;
    padding: 2.5rem 2rem 4rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  h3 {
    margin: 0 0 0.75rem;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--muted);
  }
  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 0.75rem;
  }
  .theme {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.35rem;
    padding: 0.75rem;
    border: 2px solid;
    border-radius: var(--radius);
    text-align: left;
    cursor: pointer;
  }
  .swatches {
    display: flex;
    gap: 4px;
    margin-bottom: 0.25rem;
  }
  .swatches span {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px rgb(0 0 0 / 0.12);
  }
  .theme-name {
    font-weight: 600;
  }
  .theme-kind {
    font-size: 0.8rem;
  }
  .hint {
    color: var(--muted);
    font-size: 0.85rem;
    margin: 0.75rem 0 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .row + .row {
    margin-top: 0.75rem;
  }
  input[type="color"] {
    width: 44px;
    height: 32px;
    padding: 2px;
    cursor: pointer;
  }
  input[type="range"] {
    width: 100%;
    accent-color: var(--accent);
    padding: 0;
    border: none;
    background: none;
  }
  .grid2 {
    display: grid;
    /* Wide enough for the longest translated option row; wraps to fewer columns otherwise. */
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 320px), 1fr));
    gap: 1.25rem;
  }
  .segmented {
    display: inline-flex;
    flex-wrap: wrap;
    max-width: 100%;
    background: var(--surface-2);
    border-radius: 999px;
    padding: 3px;
    gap: 2px;
  }
  .segmented button {
    border: none;
    background: transparent;
    color: var(--muted);
    border-radius: 999px;
    padding: 0.3rem 0.8rem;
    cursor: pointer;
    font: inherit;
  }
  .segmented button.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow);
  }
</style>
