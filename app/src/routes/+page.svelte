<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, type Summary } from "$lib/api";
  import { appearance } from "$lib/theme.svelte";
  import ResumeEditor from "$lib/ResumeEditor.svelte";
  import TemplateEditor from "$lib/TemplateEditor.svelte";
  import ProfileEditor from "$lib/ProfileEditor.svelte";
  import SettingsView from "$lib/SettingsView.svelte";
  import NewTemplateDialog from "$lib/NewTemplateDialog.svelte";
  import NewResumeDialog from "$lib/NewResumeDialog.svelte";

  type View =
    | { kind: "home" }
    | { kind: "profile" }
    | { kind: "settings" }
    | { kind: "resume"; id: string }
    | { kind: "template"; id: string };

  let ws = $state<Summary | null>(null);
  let view = $state<View>({ kind: "home" });
  let error = $state("");
  let creatingTemplate = $state(false);
  /** Template preselected in the new-resume dialog; `null` when closed. */
  let creatingResume = $state<string | null>(null);

  const currentResume = $derived(
    view.kind === "resume" ? ws?.resumes.find((r) => r.id === (view as { id: string }).id) : undefined,
  );
  const currentTemplate = $derived(
    view.kind === "template" ? ws?.templates.find((x) => x.id === (view as { id: string }).id) : undefined,
  );

  // Reopen the last workspace on start.
  $effect(() => {
    const last = appearance.settings.lastWorkspace;
    if (last && !ws) load(last, false);
  });

  async function load(path: string, create: boolean) {
    try {
      ws = await api.openWorkspace(path, create);
      view = { kind: "home" };
      error = "";
      if (appearance.settings.lastWorkspace !== path) appearance.update({ lastWorkspace: path });
    } catch (e) {
      error = String(e);
    }
  }

  async function pick(create: boolean) {
    const dir = await open({ directory: true, title: create ? t("welcome.pickNew") : t("welcome.open") });
    if (typeof dir === "string") load(dir, create);
  }

  async function refresh() {
    ws = await api.summary();
  }

  function initial(name: string) {
    return name.trim().charAt(0).toUpperCase() || "?";
  }
</script>

{#if !ws}
  <main class="welcome">
    <div class="glow" aria-hidden="true"></div>

    <section class="hero">
      <!-- A small stack of "paper" resumes: one template, many resumes. -->
      <div class="stack" aria-hidden="true">
        {#each [0, 1, 2] as i (i)}
          <div class="sheet" style:--i={i}>
            <div class="sheet-name"></div>
            <div class="sheet-role"></div>
            <div class="sheet-rule"></div>
            <div class="sheet-line w90"></div>
            <div class="sheet-line w75"></div>
            <div class="sheet-line w85"></div>
            <div class="sheet-rule"></div>
            <div class="sheet-line w60"></div>
            <div class="sheet-line w80"></div>
          </div>
        {/each}
      </div>

      <h1>d<span>resume</span></h1>
      <p class="tagline">{t("welcome.tagline")}</p>

      <div class="actions-row">
        <button class="primary big" onclick={() => pick(false)}>{t("welcome.open")}</button>
        <button class="big" onclick={() => pick(true)}>{t("welcome.new")}</button>
      </div>
      {#if error}<p class="error">{error}</p>{/if}

      <ul class="features">
        <li><span>▦</span> {t("welcome.featureTemplates")}</li>
        <li><span>▤</span> {t("welcome.featureContent")}</li>
        <li><span>✦</span> {t("welcome.featureAgents")}</li>
      </ul>
    </section>

    <footer class="welcome-foot">{t("welcome.footer")}</footer>
  </main>
{:else}
  <div class="shell">
    <aside>
      <button class="ws" onclick={() => (view = { kind: "home" })} title={ws.root}>
        <span class="ws-icon">{initial(ws.name)}</span>
        <span class="ws-name">{ws.name}</span>
      </button>

      <nav class="nav">
        <button class="item" class:active={view.kind === "home"} onclick={() => (view = { kind: "home" })}>
          <span class="ico">⌂</span> {t("nav.overview")} <span class="count">{ws.resumes.length}</span>
        </button>
        <button class="item" class:active={view.kind === "profile"} onclick={() => (view = { kind: "profile" })}>
          <span class="ico">☺</span> {t("nav.profile")}
        </button>
      </nav>

      <div class="section-head">
        <span>{t("nav.templates")}</span>
        <button class="ghost small" title={t("nav.newTemplate")} onclick={() => (creatingTemplate = true)}>＋</button>
      </div>

      <nav class="nav">
        {#each ws.templates as tpl (tpl.id)}
          <div class="template-head">
            <button
              class="item template-name"
              class:active={view.kind === "template" && view.id === tpl.id}
              onclick={() => (view = { kind: "template", id: tpl.id })}
              title={t("nav.editLayout")}
            >
              <span class="ico">▦</span> {tpl.name} <span class="count">{tpl.resumes.length}</span>
            </button>
            <button class="ghost small add" title={t("nav.newResumeWith")}
              onclick={() => (creatingResume = tpl.id)}>＋</button>
          </div>
          {#each tpl.resumes as r (r)}
            <button
              class="item resume"
              class:active={view.kind === "resume" && view.id === r}
              onclick={() => (view = { kind: "resume", id: r })}
            >
              {r}
            </button>
          {/each}
        {/each}
      </nav>

      <div class="spacer"></div>
      {#if error}<p class="error">{error}</p>{/if}
      <nav class="nav">
        <button class="item" class:active={view.kind === "settings"} onclick={() => (view = { kind: "settings" })}>
          <span class="ico">⚙</span> {t("nav.settings")}
        </button>
        <button class="item" onclick={() => pick(false)}>
          <span class="ico">⇄</span> {t("nav.switch")}
        </button>
      </nav>
    </aside>

    <main>
      {#if view.kind === "profile"}
        <ProfileEditor />
      {:else if view.kind === "settings"}
        <div class="scroll"><SettingsView /></div>
      {:else if currentResume}
        {#key currentResume.id}
          <ResumeEditor
            id={currentResume.id}
            template={ws.templates.find((x) => x.id === currentResume.template)?.name ?? currentResume.template}
            templateId={currentResume.template}
            ondelete={async () => { view = { kind: "home" }; await refresh(); }}
          />
        {/key}
      {:else if currentTemplate}
        {#key currentTemplate.id}
          <TemplateEditor template={currentTemplate} />
        {/key}
      {:else}
        <div class="scroll">
          <div class="home">
            <header class="page-head">
              <div class="bubble">{initial(ws.name)}</div>
              <h1>{ws.name}</h1>
              <p class="sub">{t("home.subtitle")}</p>
              <div class="meta">
                <span>{t("home.templates", { n: ws.templates.length })}</span>
                <span>{t("home.resumes", { n: ws.resumes.length })}</span>
                <code>{ws.root}</code>
              </div>
            </header>

            <div class="cards">
              {#each ws.templates as tpl (tpl.id)}
                <article class="card tcard">
                  <div class="tcard-head">
                    <h2>▦ {tpl.name}</h2>
                    <button class="ghost small" onclick={() => (view = { kind: "template", id: tpl.id })}>{t("home.editLayout")}</button>
                  </div>
                  {#if tpl.description}<p class="desc">{tpl.description}</p>{/if}
                  <ul class="resumes">
                    {#each tpl.resumes as r (r)}
                      <li>
                        <button class="resume-row" onclick={() => (view = { kind: "resume", id: r })}>
                          <span class="doc">▤</span> {r}
                        </button>
                      </li>
                    {:else}
                      <li class="none">{t("home.noResumes")}</li>
                    {/each}
                  </ul>
                  <footer>
                    <span>{t("home.resumes", { n: tpl.resumes.length })}</span>
                    <button class="ghost small" onclick={() => (creatingResume = tpl.id)}>{t("home.newResume")}</button>
                  </footer>
                </article>
              {/each}
              <button class="card new-card" onclick={() => (creatingTemplate = true)}>
                {t("home.newTemplate")}
              </button>
            </div>
          </div>
        </div>
      {/if}
    </main>
  </div>

  {#if creatingResume !== null}
    <NewResumeDialog
      templates={ws.templates}
      resumes={ws.resumes}
      template={creatingResume}
      onclose={() => (creatingResume = null)}
      oncreated={async (r) => {
        creatingResume = null;
        await refresh();
        view = { kind: "resume", id: r.id };
      }}
    />
  {/if}

  {#if creatingTemplate}
    <NewTemplateDialog
      templates={ws.templates}
      onclose={() => (creatingTemplate = false)}
      oncreated={async (created) => {
        creatingTemplate = false;
        await refresh();
        view = { kind: "template", id: created.id };
      }}
    />
  {/if}
{/if}

<style>
  .welcome {
    position: relative;
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    /* Dotted paper background, fading toward the center. */
    background-image: radial-gradient(color-mix(in srgb, var(--muted) 35%, transparent) 1px, transparent 1.4px);
    background-size: 18px 18px;
  }
  .welcome::before {
    content: "";
    position: absolute;
    inset: 0;
    background: radial-gradient(ellipse 55% 60% at 50% 48%, var(--bg) 40%, transparent 100%);
  }
  .glow {
    position: absolute;
    width: 520px;
    height: 520px;
    border-radius: 50%;
    background: radial-gradient(circle, color-mix(in srgb, var(--accent) 22%, transparent), transparent 65%);
    filter: blur(20px);
    top: 50%;
    left: 50%;
    translate: -50% -62%;
    animation: breathe 7s ease-in-out infinite;
  }
  .hero {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    animation: rise 0.7s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }

  .stack {
    position: relative;
    width: 120px;
    height: 150px;
    margin-bottom: 2rem;
  }
  .sheet {
    position: absolute;
    inset: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: var(--shadow);
    padding: 14px 12px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    transform-origin: 50% 100%;
    rotate: calc((var(--i) - 1) * 9deg);
    translate: calc((var(--i) - 1) * 26px) calc(var(--i) * -2px);
    z-index: calc(3 - var(--i));
    animation: fan 0.9s cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(0.15s + var(--i) * 0.08s);
  }
  .sheet:nth-child(2) {
    z-index: 5;
  }
  .stack:hover .sheet {
    rotate: calc((var(--i) - 1) * 14deg);
    translate: calc((var(--i) - 1) * 38px) -6px;
    transition: rotate 0.35s, translate 0.35s;
  }
  .sheet-name {
    height: 8px;
    width: 60%;
    border-radius: 4px;
    background: var(--text);
    opacity: 0.8;
    align-self: center;
  }
  .sheet-role {
    height: 4px;
    width: 40%;
    border-radius: 2px;
    background: var(--muted);
    opacity: 0.6;
    align-self: center;
    margin-bottom: 4px;
  }
  .sheet-rule {
    height: 2px;
    width: 35%;
    background: var(--accent);
    border-radius: 1px;
    margin-top: 4px;
  }
  .sheet-line {
    height: 4px;
    border-radius: 2px;
    background: var(--border);
  }
  .w90 { width: 90%; }
  .w85 { width: 85%; }
  .w80 { width: 80%; }
  .w75 { width: 75%; }
  .w60 { width: 60%; }

  .hero h1 {
    margin: 0;
    font-size: 3.4rem;
    font-weight: 800;
    letter-spacing: -0.045em;
    line-height: 1;
    color: var(--accent);
  }
  .hero h1 span {
    color: var(--text);
  }
  .tagline {
    max-width: 30ch;
    margin: 0.9rem 0 1.75rem;
    font-size: 1.1rem;
    color: var(--muted);
    text-wrap: balance;
  }
  .actions-row {
    display: flex;
    gap: 0.6rem;
  }
  button.big {
    padding: 0.6rem 1.35rem;
    font-size: 1rem;
  }
  button.big.primary {
    box-shadow: 0 6px 20px -6px color-mix(in srgb, var(--accent) 70%, transparent);
  }
  button.big:hover:not(:disabled) {
    translate: 0 -1px;
  }
  .features {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.5rem;
    margin: 2.25rem 0 0;
    padding: 0;
  }
  .features li {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.8rem;
    border-radius: 999px;
    background: color-mix(in srgb, var(--surface) 70%, transparent);
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.85rem;
    backdrop-filter: blur(6px);
  }
  .features span {
    color: var(--accent);
  }
  .welcome-foot {
    position: absolute;
    bottom: 1.25rem;
    font-size: 0.8rem;
    color: var(--muted);
    opacity: 0.8;
  }

  @keyframes rise {
    from { opacity: 0; translate: 0 14px; }
  }
  @keyframes fan {
    from { rotate: 0deg; translate: 0 10px; opacity: 0; }
  }
  @keyframes breathe {
    50% { scale: 1.08; opacity: 0.8; }
  }
  @media (prefers-reduced-motion: reduce) {
    .hero, .sheet, .glow { animation: none; }
  }
  .shell {
    display: grid;
    grid-template-columns: 240px 1fr;
    height: 100%;
  }
  aside {
    background: var(--sidebar);
    border-right: 1px solid var(--border);
    overflow: auto;
    padding: 0.75rem 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .ws {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    border: none;
    background: transparent;
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.5rem;
    margin-bottom: 0.75rem;
    text-align: left;
  }
  .ws-icon {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 700;
    flex-shrink: 0;
  }
  .ws-name {
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .section-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 1.25rem;
    padding: 0 0.25rem 0.25rem 0.6rem;
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    border-radius: var(--radius-sm);
    padding: 0.38rem 0.6rem;
    color: var(--text);
  }
  .item:hover:not(:disabled) {
    background: var(--surface-2);
  }
  .item.active {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  .ico {
    width: 1.1rem;
    text-align: center;
    color: var(--muted);
  }
  .item.active .ico {
    color: var(--accent);
  }
  .count {
    margin-left: auto;
    font-size: 0.8rem;
    color: var(--muted);
    font-weight: 400;
  }
  .template-head {
    display: flex;
    align-items: center;
    margin-top: 0.35rem;
  }
  .template-head .add {
    opacity: 0;
  }
  .template-head:hover .add {
    opacity: 1;
  }
  .template-name {
    font-weight: 600;
  }
  .resume {
    padding-left: 2.2rem;
    color: var(--muted);
  }
  .spacer {
    flex: 1;
    min-height: 1rem;
  }
  main {
    min-width: 0;
    min-height: 0;
    height: 100%;
    background: var(--bg);
  }
  .scroll {
    height: 100%;
    overflow: auto;
  }
  .home {
    max-width: 1100px;
    margin: 0 auto;
    padding: 2.5rem 2rem 4rem;
  }
  .meta code {
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 1rem;
    margin-top: 1.25rem;
    align-items: start;
  }
  .tcard {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .tcard-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .tcard h2 {
    margin: 0;
    font-size: 1.25rem;
    letter-spacing: -0.01em;
  }
  .desc {
    margin: 0;
    color: var(--muted);
  }
  .resumes {
    list-style: none;
    margin: 0.25rem 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .resume-row {
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    border-radius: var(--radius-sm);
    padding: 0.35rem 0.5rem;
    display: flex;
    gap: 0.5rem;
  }
  .doc {
    color: var(--accent);
  }
  .none {
    color: var(--muted);
    font-style: italic;
    padding: 0.35rem 0.5rem;
  }
  .tcard footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 0.25rem;
    padding-top: 0.6rem;
    border-top: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.85rem;
  }
  .new-card {
    min-height: 140px;
    border-style: dashed;
    box-shadow: none;
    color: var(--muted);
    background: transparent;
    border-radius: var(--radius);
  }
  .error {
    color: var(--danger);
    font-size: 0.85rem;
    padding: 0 0.5rem;
  }
</style>
