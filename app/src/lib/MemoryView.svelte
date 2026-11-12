<script lang="ts">
  import { t } from "./i18n.svelte";
  import { api, type MemoryStatus } from "./api";
  import { appearance } from "./theme.svelte";
  import CodeEditor from "./CodeEditor.svelte";
  import SplitPane from "./SplitPane.svelte";
  import { onDestroy } from "svelte";

  let text = $state("");
  let saved = $state("");
  let status = $state<MemoryStatus | null>(null);
  let message = $state("");
  let compacting = $state(false);

  const dirty = $derived(text !== saved);
  /** Live size while editing (UTF-8 bytes, like the core). */
  const size = $derived(new TextEncoder().encode(text).length);
  const limit = $derived(status?.limit ?? 4096);

  async function load() {
    const [m, s] = await api.getMemory();
    text = saved = m;
    status = s;
  }

  $effect(() => {
    load().catch((e) => flash(String(e)));
  });

  // The agent saves facts while chatting; show them as they land (unless mid-edit).
  const unlisten = api.onWorkspaceChanged((paths) => {
    if (paths.includes("memory.md") && !dirty) load();
  });
  onDestroy(() => unlisten.then((f) => f()));

  function flash(m: string) {
    message = m;
    setTimeout(() => (message = ""), 3000);
  }

  async function persist() {
    try {
      status = await api.saveMemory(text);
      saved = text;
      flash(t("common.saved"));
    } catch (e) {
      flash(String(e));
    }
  }

  async function compact() {
    if (dirty) await persist();
    const out = await api.compactMemory();
    await load();
    flash(t("memory.compacted", { n: out.removed.length }));
  }

  /** Runs a one-off agent session that rewrites memory.md, then keeps its changes. */
  async function compactWithAgent() {
    const agents = (await api.listAgents()).filter((a) => a.path);
    const kind = agents.find((a) => a.kind === appearance.settings.agent)?.kind ?? agents[0]?.kind;
    if (!kind) return flash(t("memory.noAgent"));
    if (dirty) await persist();
    compacting = true;
    let session: number | null = null;
    const done = new Promise<string | null>((resolve) => {
      api
        .agentStart(kind, "autoEdit", appearance.settings.agentModel[kind] ?? null, appearance.settings.agentEffort[kind] ?? null, { kind: "memory" }, false, (e) => {
          if (e.type === "turn_end") resolve(e.error);
          else if (e.type === "error") resolve(e.message);
          else if (e.type === "exited") resolve(null);
        })
        .then(async (id) => {
          session = id;
          await api.agentSend(
            id,
            `Compact memory.md: merge facts that say the same thing (the newer one wins), drop facts that are obsolete, ` +
              `keep 📌 lines, the sections and the "- fact (YYYY-MM-DD)" format, and stay under ${limit} bytes. ` +
              `Edit the file directly. Don't add facts.`,
          );
        })
        .catch((e) => resolve(String(e)));
    });
    const error = await done;
    if (session !== null) {
      await api.agentKeep(session).catch(() => {});
      await api.agentStop(session);
    }
    compacting = false;
    await load();
    flash(error ?? t("common.saved"));
  }

  function onkeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      persist();
    }
  }
</script>

<svelte:window {onkeydown} />

<section class="memory">
  <header class="topbar">
    <div class="crumbs">
      <span class="chip">✦ {t("memory.title")}{dirty ? " •" : ""}</span>
      <span class="chip muted">{t("memory.subtitle")}</span>
    </div>
    <div class="actions">
      {#if message}<span class="flash">{message}</span>{/if}
      <button class="primary" onclick={persist} disabled={!dirty}>{t("common.save")}</button>
    </div>
  </header>

  <div class="body">
    <SplitPane id="memory" initial={0.6} min={260}>
      {#snippet left()}
        <div class="code">
          <CodeEditor name="memory.md" value={text} onchange={(v) => (text = v)} />
        </div>
      {/snippet}
      {#snippet right()}
        <div class="side">
          {#if !appearance.settings.memory}
            <p class="off">{t("memory.off")}</p>
          {/if}

          <div class="card">
            <h3>{t("memory.usage", { size, limit })}</h3>
            <div class="meter" class:over={size > limit}>
              <div style:width="{Math.min(100, (size / limit) * 100)}%"></div>
            </div>
            {#if size > limit}<p class="warn">{t("memory.over")}</p>{/if}
            <p class="hint">{t("memory.hint")}</p>
          </div>

          <div class="card">
            <button onclick={compact} disabled={compacting}>{t("memory.compact")}</button>
            <p class="hint">{t("memory.compactHint")}</p>
            <button onclick={compactWithAgent} disabled={compacting}>
              ✦ {compacting ? t("memory.compacting") : t("memory.compactAgent")}
            </button>
            <p class="hint">{t("memory.compactAgentHint")}</p>
          </div>

          {#if status?.similar.length}
            <div class="card">
              <h3>{t("memory.similar")}</h3>
              <ul class="pairs">
                {#each status.similar as [a, b], i (i)}
                  <li><span>{a}</span><span class="tilde">≈</span><span>{b}</span></li>
                {/each}
              </ul>
            </div>
          {/if}
        </div>
      {/snippet}
    </SplitPane>
  </div>
</section>

<style>
  .memory {
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
    margin: 0 0 0.5rem;
    font-size: 1rem;
  }
  .hint {
    margin: 0.5rem 0 0.75rem;
    color: var(--muted);
    font-size: 0.88rem;
  }
  .hint:last-child {
    margin-bottom: 0;
  }
  .meter {
    height: 0.5rem;
    border-radius: 999px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .meter div {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s;
  }
  .meter.over div {
    background: var(--danger);
  }
  .warn {
    margin: 0.5rem 0 0;
    color: var(--danger);
    font-size: 0.88rem;
  }
  .off {
    margin: 0;
    padding: 0.6rem 0.8rem;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--warn) 15%, var(--surface));
    font-size: 0.88rem;
  }
  .pairs {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    font-size: 0.88rem;
  }
  .pairs li {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .tilde {
    color: var(--muted);
  }
  @media (prefers-reduced-motion: reduce) {
    .meter div {
      transition: none;
    }
  }
</style>
