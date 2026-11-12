<script lang="ts">
  import SaveButton from "./SaveButton.svelte";
  import { t } from "./i18n.svelte";
  import { api, type ProfileVisibility } from "./api";
  import CodeEditor from "./CodeEditor.svelte";
  import SplitPane from "./SplitPane.svelte";

  /** Fields every template always receives (mirrors ALWAYS_SHOWN in core). */
  const ALWAYS_SHOWN = ["name"];

  let text = $state("");
  let saved = $state("");
  let message = $state("");
  let visibility = $state<ProfileVisibility | null>(null);

  $effect(() => {
    api.getProfile().then((p) => (text = saved = p));
    loadVisibility();
  });

  async function loadVisibility() {
    try {
      visibility = await api.profileVisibility();
    } catch (e) {
      message = String(e);
    }
  }

  function flash(m: string) {
    message = m;
    setTimeout(() => (message = ""), 3000);
  }

  async function persist() {
    try {
      await api.saveProfile(text);
      saved = text;
      flash(t("profile.saved"));
      // New or renamed fields show up in the visibility table.
      await loadVisibility();
    } catch (e) {
      flash(String(e));
    }
  }

  async function toggle(template: string, field: string, show: boolean) {
    const tpl = visibility?.templates.find((x) => x.id === template);
    if (!tpl) return;
    const hidden = show ? tpl.hidden.filter((f) => f !== field) : [...tpl.hidden, field];
    tpl.hidden = hidden;
    try {
      await api.setProfileHidden(template, hidden);
    } catch (e) {
      flash(String(e));
      await loadVisibility();
    }
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
      <SaveButton onclick={persist} disabled={text === saved} />
    </div>
  </header>

  <div class="body">
    <SplitPane id="profile" initial={0.55} min={260}>
      {#snippet left()}
        <div class="code">
          <CodeEditor name="profile.yaml" value={text} onchange={(v) => (text = v)} />
        </div>
      {/snippet}
      {#snippet right()}
        <div class="side">
          <div class="card">
            <h3>{t("profile.visibility")}</h3>
            <p class="hint">{t("profile.visibilityHint")}</p>

            {#if visibility && visibility.templates.length && visibility.fields.length}
              <div class="table-wrap">
                <table>
                  <thead>
                    <tr>
                      <th></th>
                      {#each visibility.templates as tpl (tpl.id)}<th title={tpl.id}>{tpl.name}</th>{/each}
                    </tr>
                  </thead>
                  <tbody>
                    {#each visibility.fields as field (field)}
                      {@const locked = ALWAYS_SHOWN.includes(field)}
                      <tr>
                        <td class="field"><code>{field}</code></td>
                        {#each visibility.templates as tpl (tpl.id)}
                          <td class="check">
                            <input
                              type="checkbox"
                              checked={locked || !tpl.hidden.includes(field)}
                              disabled={locked}
                              title={locked ? t("profile.alwaysShown") : ""}
                              onchange={(e) => toggle(tpl.id, field, e.currentTarget.checked)}
                            />
                          </td>
                        {/each}
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {:else if visibility}
              <p class="hint">{t("profile.noTemplates")}</p>
            {/if}
          </div>

          <div class="card tips">
            <h3>{t("profile.tipsTitle")}</h3>
            <p class="hint">{t("profile.tipLinks")}</p>
            <pre><code>website: https://you.dev
github:
  label: github.com/you
  url: https://github.com/you</code></pre>
          </div>
        </div>
      {/snippet}
    </SplitPane>
  </div>
</section>

<style>
  .profile {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .body {
    flex: 1;
    min-height: 0;
  }
  .code {
    height: 100%;
  }
  .side {
    height: 100%;
    overflow: auto;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    box-sizing: border-box;
  }
  h3 {
    margin: 0 0 0.25rem;
    font-size: 1rem;
  }
  .hint {
    margin: 0 0 0.75rem;
    color: var(--muted);
    font-size: 0.88rem;
  }
  .table-wrap {
    overflow-x: auto;
  }
  table {
    border-collapse: collapse;
    width: 100%;
    font-size: 0.9rem;
  }
  th {
    font-weight: 600;
    color: var(--muted);
    font-size: 0.8rem;
    padding: 0.4rem 0.5rem;
    text-align: center;
    white-space: nowrap;
  }
  td {
    border-top: 1px solid var(--border);
    padding: 0.35rem 0.5rem;
  }
  td.field code {
    color: var(--text);
  }
  td.check {
    text-align: center;
  }
  td.check input {
    width: 1.05rem;
    height: 1.05rem;
    accent-color: var(--accent);
    cursor: pointer;
  }
  td.check input:disabled {
    cursor: default;
    opacity: 0.5;
  }
  tbody tr:hover td {
    background: var(--surface-2);
  }
  pre {
    margin: 0;
    padding: 0.75rem;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
    font-size: 0.82rem;
    overflow-x: auto;
  }
</style>
