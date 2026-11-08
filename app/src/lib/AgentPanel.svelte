<script lang="ts">
  import { onDestroy, tick, untrack } from "svelte";
  import { t } from "./i18n.svelte";
  import { appearance } from "./theme.svelte";
  import { api, type AgentEvent, type AgentInfo, type AgentKind, type ToolStatus } from "./api";

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
    | { kind: "permission"; id: string; title: string; detail: string | null; answer: boolean | null };

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

  /** Incremented on every new session so late events from a stopped one are ignored. */
  let generation = 0;

  $effect(() => {
    const key = resume;
    untrack(() => {
      stop();
      messages = [];
    });
    api.loadChat<Msg>(key).then((saved) => {
      if (key === resume) messages = saved;
      scrollDown();
    }).catch(() => {});
  });

  onDestroy(stop);

  function stop() {
    generation++;
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
        persist();
        onturnend?.();
        break;
      case "error":
        messages.push({ kind: "error", text: e.message });
        break;
      case "exited":
        if (status !== "idle") messages.push({ kind: "error", text: t("agent.exited") });
        session = null;
        status = "idle";
        persist();
        break;
    }
    scrollDown();
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
        const id = await api.agentStart(kind, "autoEdit", resume, (e) => mine === generation && onEvent(e));
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
    stop();
    appearance.update({ agent: next });
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      send();
    }
  }
</script>

<aside class="agent">
  <header>
    <strong>✦ {t("agent.title")}</strong>
    {#if installed.length > 1}
      <select value={kind} onchange={(e) => pick(e.currentTarget.value as AgentKind)}>
        {#each installed as a (a.kind)}<option value={a.kind}>{a.name}</option>{/each}
      </select>
    {:else if kind}
      <span class="muted">{installed[0].name}</span>
    {/if}
    <button class="ghost small" onclick={newChat} disabled={!messages.length}>{t("agent.newChat")}</button>
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
        {/if}
      {/each}
      {#if status !== "idle"}
        <p class="muted working">{status === "starting" ? t("agent.starting") : t("agent.working")}</p>
      {/if}
    </div>

    <form class="compose" onsubmit={(e) => { e.preventDefault(); send(); }}>
      <textarea rows="3" placeholder={t("agent.placeholder")} bind:value={draft} {onkeydown}></textarea>
      <div class="row">
        {#if status === "working" && session !== null}
          <button type="button" class="small" onclick={() => session !== null && api.agentCancel(session)}>{t("agent.stop")}</button>
        {/if}
        <button type="submit" class="small primary" disabled={!draft.trim() || status !== "idle"}>{t("agent.send")}</button>
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
    gap: 0.5rem;
    padding: 0.6rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }
  header strong {
    margin-right: auto;
  }
  header select {
    max-width: 9rem;
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
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 0.4rem;
  }
  .working {
    font-size: 0.85rem;
  }
  .compose {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.6rem 0.75rem 0.75rem;
    border-top: 1px solid var(--border);
  }
  .compose textarea {
    resize: vertical;
    min-height: 3.5rem;
    max-height: 40vh;
  }
</style>
