<script lang="ts">
  import { onDestroy, tick, untrack } from "svelte";
  import { t } from "./i18n.svelte";
  import { appearance } from "./theme.svelte";
  import { layout } from "./layout.svelte";
  import {
    api,
    type AgentEvent,
    type AgentInfo,
    type AgentKind,
    type FileChange,
    type ModelOption,
    type PermissionMode,
    type ToolStatus,
  } from "./api";
  import type { Key } from "./i18n.svelte";

  interface Props {
    /** Resume the user has open; the agent is told about it and the chat is kept per resume. */
    resume: string;
    /** Called when a turn ends, so the editor can pick up changed files. */
    onturnend?: () => void;
  }

  let { resume, onturnend }: Props = $props();

  type Msg =
    | { kind: "user" | "assistant" | "error"; text: string }
    | { kind: "tool"; id: string; title: string; status: ToolStatus }
    | { kind: "permission"; id: string; title: string; detail: string | null; answer: boolean | null }
    | { kind: "changes"; session: number; files: Changed[]; state: "pending" | "kept" | "undone" | "expired" };

  type Changed = { path: string; diff: string; status: "new" | "deleted" | "edited" };

  let agents = $state<AgentInfo[] | null>(null);
  let messages = $state<Msg[]>([]);
  let draft = $state("");
  let session = $state<number | null>(null);
  let status = $state<"idle" | "starting" | "working">("idle");
  let scroller = $state<HTMLElement>();

  const installed = $derived((agents ?? []).filter((a) => a.path));
  const kind = $derived<AgentKind | null>(
    installed.find((a) => a.kind === appearance.settings.agent)?.kind ?? installed[0]?.kind ?? null,
  );

  api.listAgents().then((list) => (agents = list)).catch(() => (agents = []));

  // Model and effort are remembered per agent; `null` keeps the agent's own default.
  let models = $state<ModelOption[]>([]);
  $effect(() => {
    const k = kind;
    models = [];
    if (k) api.agentModels(k).then((list) => k === kind && (models = list)).catch(() => {});
  });
  const model = $derived(models.find((m) => kind && m.id === appearance.settings.agentModel[kind]) ?? null);
  /** Efforts of the chosen model; with the default model, those of the first one. */
  const efforts = $derived((model ?? models[0])?.efforts ?? []);
  const effort = $derived.by(() => {
    const e = kind ? appearance.settings.agentEffort[kind] : undefined;
    return e && efforts.includes(e) ? e : null;
  });

  const modes: { id: PermissionMode; label: Key; desc: Key }[] = [
    { id: "ask", label: "agent.modeAsk", desc: "agent.modeAskDesc" },
    { id: "autoEdit", label: "agent.modeAutoEdit", desc: "agent.modeAutoEditDesc" },
    { id: "readOnly", label: "agent.modeReadOnly", desc: "agent.modeReadOnlyDesc" },
  ];
  const mode = $derived(modes.find((m) => m.id === appearance.settings.agentMode) ?? modes[1]);

  /** Which popover menu is open. */
  let menu = $state<"agent" | "model" | "mode" | null>(null);

  function setModel(id: string | null) {
    if (!kind) return;
    appearance.update({ agentModel: { ...appearance.settings.agentModel, [kind]: id } });
  }

  function setEffort(e: string | null) {
    if (!kind) return;
    appearance.update({ agentEffort: { ...appearance.settings.agentEffort, [kind]: e } });
  }

  /** Effort ids come from the agent ("xhigh", "max"…); shown capitalized. */
  const effortName = (e: string) => (e === "xhigh" ? "X-High" : e[0].toUpperCase() + e.slice(1));

  /** Incremented on every new session so late events from a stopped one are ignored. */
  let generation = 0;
  /** Sessions whose process exited; kept alive in the backend for undo until the chat moves on. */
  let dead: number[] = [];

  // A new agent, model, effort or permission mode applies from the next session.
  $effect(() => {
    void kind;
    void model;
    void effort;
    void appearance.settings.agentMode;
    untrack(stop);
  });

  $effect(() => {
    const key = resume;
    untrack(() => {
      stop();
      messages = [];
    });
    api.loadChat<Msg>(key).then((saved) => {
      // Undo needs the session that made the changes, which is gone after a reload.
      for (const m of saved) if (m.kind === "changes" && m.state === "pending") m.state = "expired";
      if (key === resume) messages = saved;
      scrollDown();
    }).catch(() => {});
  });

  onDestroy(stop);

  function stop() {
    generation++;
    for (const id of dead.splice(0)) api.agentStop(id);
    if (session !== null) api.agentStop(session);
    session = null;
    status = "idle";
  }

  function persist() {
    // Unanswered requests die with the session, so they're dropped from the history.
    const kept = messages.filter((m) => m.kind !== "permission" || m.answer !== null);
    api.saveChat(resume, $state.snapshot(kept)).catch(() => {});
  }

  async function scrollDown() {
    await tick();
    scroller?.scrollTo({ top: scroller.scrollHeight });
  }

  function onEvent(e: AgentEvent) {
    const last = messages.at(-1);
    switch (e.type) {
      case "text":
        if (last?.kind === "assistant") last.text += e.text;
        else messages.push({ kind: "assistant", text: e.text });
        break;
      case "tool": {
        const tool = messages.findLast((m) => m.kind === "tool" && m.id === e.id);
        if (tool?.kind === "tool") {
          tool.status = e.status;
          if (e.title) tool.title = e.title;
        } else {
          messages.push({ kind: "tool", id: e.id, title: e.title, status: e.status });
        }
        break;
      }
      case "permission":
        messages.push({ kind: "permission", id: e.id, title: e.title, detail: e.detail, answer: null });
        break;
      case "turn_end":
        if (e.error) messages.push({ kind: "error", text: e.error });
        status = "idle";
        if (session !== null) showChanges(session);
        else persist();
        onturnend?.();
        break;
      case "error":
        messages.push({ kind: "error", text: e.message });
        break;
      case "exited":
        if (status !== "idle") messages.push({ kind: "error", text: t("agent.exited") });
        if (session !== null) dead.push(session);
        session = null;
        status = "idle";
        persist();
        break;
    }
    scrollDown();
  }

  /** Lists what the agent changed since the last keep/undo; changes add up across turns. */
  async function showChanges(id: number) {
    const files = await api.agentChanges(id).catch(() => [] as FileChange[]);
    if (files.length) {
      messages = messages.filter((m) => m.kind !== "changes" || m.state !== "pending");
      messages.push({
        kind: "changes",
        session: id,
        state: "pending",
        files: files.map((f) => ({
          path: f.path,
          diff: f.diff,
          status: f.before === null ? "new" : f.after === null ? "deleted" : "edited",
        })),
      });
      scrollDown();
    }
    persist();
  }

  async function decide(msg: Msg, keep: boolean) {
    if (msg.kind !== "changes" || msg.state !== "pending") return;
    try {
      if (keep) await api.agentKeep(msg.session);
      else await api.agentUndo(msg.session);
      msg.state = keep ? "kept" : "undone";
    } catch (e) {
      messages.push({ kind: "error", text: String(e) });
    }
    persist();
  }

  /** Diff lines with their kind, for coloring. */
  function lines(diff: string) {
    return diff
      .split("\n")
      .filter((l) => !l.startsWith("---") && !l.startsWith("+++"))
      .map((l) => ({ text: l, kind: l.startsWith("+") ? "add" : l.startsWith("-") ? "del" : l.startsWith("@@") ? "hunk" : "" }));
  }

  async function send() {
    const text = draft.trim();
    if (!text || !kind || status !== "idle") return;
    draft = "";
    messages.push({ kind: "user", text });
    scrollDown();
    try {
      if (session === null) {
        status = "starting";
        const mine = ++generation;
        const mode = appearance.settings.agentMode;
        const id = await api.agentStart(kind, mode, model?.id ?? null, effort, resume, (e) => mine === generation && onEvent(e));
        if (mine !== generation) return void api.agentStop(id);
        session = id;
      }
      status = "working";
      await api.agentSend(session, text);
    } catch (e) {
      messages.push({ kind: "error", text: String(e) });
      status = "idle";
    }
  }

  function answer(msg: Msg, allow: boolean) {
    if (msg.kind !== "permission" || session === null) return;
    msg.answer = allow;
    api.agentRespond(session, msg.id, allow);
  }

  function newChat() {
    stop();
    messages = [];
    persist();
  }

  function pick(next: AgentKind) {
    menu = null;
    appearance.update({ agent: next });
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      send();
    }
  }
</script>

<svelte:window onclick={(e) => { if (!(e.target as Element).closest?.(".menu-wrap")) menu = null; }} />

<aside class="agent">
  <header>
    <div class="menu-wrap">
      <button class="ghost head-btn" onclick={() => (menu = menu === "agent" ? null : "agent")} disabled={installed.length < 2}>
        <span class="spark">✦</span> {installed.find((a) => a.kind === kind)?.name ?? t("agent.title")}{#if installed.length > 1} ▾{/if}
      </button>
      {#if menu === "agent"}
        <div class="menu down" role="menu">
          {#each installed as a (a.kind)}
            <button class:active={a.kind === kind} onclick={() => pick(a.kind)}>{a.name}<small>{a.version ?? ""}</small></button>
          {/each}
        </div>
      {/if}
    </div>
    <span class="grow"></span>
    <button class="ghost icon" onclick={newChat} disabled={!messages.length} title={t("agent.newChat")} aria-label={t("agent.newChat")}>✎</button>
    <button class="ghost icon" onclick={() => layout.toggleAgent()} title={t("agent.close")} aria-label={t("agent.close")}>✕</button>
  </header>

  {#if agents && !installed.length}
    <div class="empty">
      <p><strong>{t("agent.none")}</strong></p>
      <p class="muted">{t("agent.noneHint")}</p>
    </div>
  {:else}
    <div class="log" bind:this={scroller}>
      {#if !messages.length}<p class="muted hint">{t("agent.empty")}</p>{/if}
      {#each messages as msg, i (i)}
        {#if msg.kind === "user"}
          <p class="bubble-user">{msg.text}</p>
        {:else if msg.kind === "assistant"}
          <p class="text">{msg.text}</p>
        {:else if msg.kind === "error"}
          <p class="error">{msg.text}</p>
        {:else if msg.kind === "tool"}
          <p class="tool {msg.status}" title={msg.title}>
            <span class="dot"></span><code>{msg.title}</code>
          </p>
        {:else if msg.kind === "permission"}
          <div class="permission">
            <span class="muted">{t("agent.permission")}</span>
            <code>{msg.title}</code>
            {#if msg.detail}<pre>{msg.detail}</pre>{/if}
            {#if msg.answer === null}
              <div class="row">
                <button class="small" onclick={() => answer(msg, false)}>{t("agent.deny")}</button>
                <button class="small primary" onclick={() => answer(msg, true)}>{t("agent.allow")}</button>
              </div>
            {:else}
              <span class="muted">{msg.answer ? t("agent.allowed") : t("agent.denied")}</span>
            {/if}
          </div>
        {:else if msg.kind === "changes"}
          <div class="changes">
            <strong>{t("agent.changes", { n: msg.files.length })}</strong>
            {#each msg.files as f (f.path)}
              <details>
                <summary><code>{f.path}</code>{#if f.status !== "edited"} <span class="tag">{f.status === "new" ? t("agent.created") : t("agent.deleted")}</span>{/if}</summary>
                <pre class="diff">{#each lines(f.diff) as l, j (j)}<span class={l.kind}>{l.text}</span>{"\n"}{/each}</pre>
              </details>
            {/each}
            {#if msg.state === "pending"}
              <div class="row">
                <button class="small" onclick={() => decide(msg, false)}>{t("agent.undo")}</button>
                <button class="small primary" onclick={() => decide(msg, true)}>{t("agent.keep")}</button>
              </div>
            {:else if msg.state !== "expired"}
              <span class="muted">{msg.state === "kept" ? t("agent.kept") : t("agent.undone")}</span>
            {/if}
          </div>
        {/if}
      {/each}
      {#if status !== "idle"}
        <p class="muted working">{status === "starting" ? t("agent.starting") : t("agent.working")}</p>
      {/if}
    </div>

    <form class="composer" onsubmit={(e) => { e.preventDefault(); send(); }}>
      <textarea rows="2" placeholder={t("agent.placeholder")} bind:value={draft} {onkeydown}></textarea>
      <div class="bar">
        <div class="menu-wrap">
          <button type="button" class="chip-btn" title={t(mode.desc)} onclick={() => (menu = menu === "mode" ? null : "mode")}>
            {t(mode.label)} ▾
          </button>
          {#if menu === "mode"}
            <div class="menu" role="menu">
              <span class="menu-label">{t("agent.mode")}</span>
              {#each modes as m (m.id)}
                <button type="button" class:active={m.id === mode.id} onclick={() => { appearance.update({ agentMode: m.id }); menu = null; }}>
                  {t(m.label)}<small>{t(m.desc)}</small>
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <span class="grow"></span>
        <div class="menu-wrap">
          <button type="button" class="chip-btn model" onclick={() => (menu = menu === "model" ? null : "model")}>
            <span class="spark">✦</span>
            {model?.name ?? t("agent.defaultModel")}{#if effort}<span class="effort">{effortName(effort)}</span>{/if} ▾
          </button>
          {#if menu === "model"}
            <div class="menu right" role="menu">
              <span class="menu-label">{t("agent.model")}</span>
              <button type="button" class:active={!model} onclick={() => setModel(null)}>{t("agent.defaultModel")}</button>
              {#each models as m (m.id)}
                <button type="button" class:active={model?.id === m.id} onclick={() => setModel(m.id)}>{m.name}</button>
              {/each}
              {#if efforts.length}
                <span class="menu-label">{t("agent.effort")}</span>
                <div class="efforts">
                  <button type="button" class:active={!effort} onclick={() => setEffort(null)}>{t("agent.effortAuto")}</button>
                  {#each efforts as e (e)}
                    <button type="button" class:active={effort === e} onclick={() => setEffort(e)}>{effortName(e)}</button>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        </div>
        {#if status === "working" && session !== null}
          <button type="button" class="send" title={t("agent.stop")} aria-label={t("agent.stop")} onclick={() => session !== null && api.agentCancel(session)}>■</button>
        {:else}
          <button type="submit" class="send" title={t("agent.send")} aria-label={t("agent.send")} disabled={!draft.trim() || status !== "idle"}>↑</button>
        {/if}
      </div>
    </form>
  {/if}
</aside>

<style>
  .agent {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--sidebar);
    border-left: 1px solid var(--border);
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.4rem 0.5rem 0.4rem 0.6rem;
    border-bottom: 1px solid var(--border);
  }
  .grow {
    flex: 1;
  }
  .head-btn {
    font-weight: 600;
  }
  .head-btn:disabled {
    opacity: 1;
    cursor: default;
  }
  .spark {
    color: var(--accent);
  }
  .icon {
    width: 2rem;
    height: 2rem;
    padding: 0;
    display: grid;
    place-items: center;
  }
  .muted {
    color: var(--muted);
  }
  .empty {
    padding: 1rem;
  }
  .log {
    flex: 1;
    overflow: auto;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .log p {
    margin: 0;
  }
  .hint {
    font-size: 0.85rem;
  }
  .text,
  .bubble-user,
  .error {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    line-height: 1.45;
  }
  .bubble-user {
    align-self: flex-end;
    max-width: 90%;
    background: var(--accent-soft);
    color: var(--text);
    padding: 0.45rem 0.7rem;
    border-radius: var(--radius);
  }
  .error {
    color: var(--danger);
    background: var(--danger-bg);
    padding: 0.4rem 0.6rem;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
  }
  .tool {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    color: var(--muted);
    min-width: 0;
  }
  .tool code {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
  }
  .dot {
    flex: none;
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 50%;
    background: var(--muted);
  }
  .tool.running .dot {
    background: var(--accent);
    animation: pulse 1s ease-in-out infinite alternate;
  }
  .tool.done .dot {
    background: color-mix(in srgb, var(--accent) 60%, var(--muted));
  }
  .tool.failed .dot {
    background: var(--danger);
  }
  @keyframes pulse {
    to {
      opacity: 0.3;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .tool.running .dot {
      animation: none;
    }
  }
  .permission {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.6rem;
    background: var(--surface);
    border: 1px solid color-mix(in srgb, var(--warn) 50%, var(--border));
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
  }
  .permission code {
    font-family: var(--mono);
    overflow-wrap: anywhere;
  }
  .permission pre {
    margin: 0;
    max-height: 12rem;
    overflow: auto;
    padding: 0.4rem;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
    font-family: var(--mono);
    font-size: 0.75rem;
    white-space: pre-wrap;
  }
  .changes {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.6rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
  }
  .changes summary {
    cursor: pointer;
  }
  .changes code {
    font-family: var(--mono);
  }
  .tag {
    font-size: 0.75rem;
    color: var(--muted);
  }
  .diff {
    margin: 0.3rem 0 0;
    max-height: 16rem;
    overflow: auto;
    padding: 0.4rem;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
    font-family: var(--mono);
    font-size: 0.75rem;
  }
  .diff .add {
    color: color-mix(in srgb, var(--accent) 80%, var(--text));
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .diff .del {
    color: var(--danger);
    background: var(--danger-bg);
  }
  .diff .hunk {
    color: var(--muted);
  }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 0.4rem;
  }
  .working {
    font-size: 0.85rem;
  }
  .composer {
    margin: 0.5rem 0.6rem 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.55rem 0.55rem 0.45rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: calc(var(--radius) * 1.4);
    box-shadow: var(--shadow);
  }
  .composer:focus-within {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  }
  .composer textarea {
    border: none;
    background: transparent;
    box-shadow: none;
    outline: none;
    resize: none;
    padding: 0.2rem 0.3rem;
    min-height: 2.6rem;
    max-height: 40vh;
    field-sizing: content;
    font: inherit;
    color: var(--text);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
  .chip-btn {
    border: none;
    background: transparent;
    color: var(--muted);
    padding: 0.25rem 0.5rem;
    font-size: 0.8rem;
    border-radius: 999px;
    white-space: nowrap;
  }
  .chip-btn:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .chip-btn.model {
    color: var(--text);
  }
  .effort {
    margin-left: 0.3rem;
    color: var(--muted);
  }
  .send {
    flex: none;
    width: 2rem;
    height: 2rem;
    padding: 0;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--text);
    color: var(--surface);
    border: none;
    font-weight: 700;
  }
  .send:disabled {
    background: var(--surface-2);
    color: var(--muted);
  }
  .menu-wrap {
    position: relative;
  }
  .menu {
    position: absolute;
    bottom: calc(100% + 0.35rem);
    left: 0;
    z-index: 20;
    min-width: 12rem;
    max-width: 18rem;
    max-height: 50vh;
    overflow: auto;
    display: flex;
    flex-direction: column;
    padding: 0.3rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
  }
  .menu.right {
    left: auto;
    right: 0;
  }
  .menu.down {
    bottom: auto;
    top: calc(100% + 0.25rem);
  }
  .menu > button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.1rem;
    border: none;
    background: transparent;
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.55rem;
    text-align: left;
    font-size: 0.85rem;
    color: var(--text);
  }
  .menu > button:hover,
  .menu > button.active {
    background: var(--surface-2);
  }
  .menu small {
    color: var(--muted);
    font-size: 0.75rem;
  }
  .menu-label {
    padding: 0.4rem 0.55rem 0.2rem;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .efforts {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
    padding: 0.1rem 0.4rem 0.3rem;
  }
  .efforts button {
    border: 1px solid var(--border);
    background: transparent;
    border-radius: 999px;
    padding: 0.15rem 0.55rem;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .efforts button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
