<script lang="ts">
  import { t } from "./i18n.svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { api, debounce, type Diagnostic, type Json, type Schema, type SchemaError } from "./api";
  import ResumeForm from "./form/ResumeForm.svelte";
  import Preview from "./Preview.svelte";
  import SplitPane from "./SplitPane.svelte";
  import ViewSwitch from "./ViewSwitch.svelte";
  import AgentPane from "./AgentPane.svelte";
  import { onDestroy } from "svelte";
  import { isShortcut, layout } from "./layout.svelte";

  interface Props {
    id: string;
    /** Template display name. */
    template: string;
    templateId: string;
    ondelete: () => void;
    onrename: () => void;
  }

  let { id, template, templateId, ondelete, onrename }: Props = $props();

  let schema = $state<Schema | null>(null);
  let value = $state<Record<string, Json>>({});
  let dirty = $state(false);
  let errors = $state<SchemaError[]>([]);
  let svg = $state<string[]>([]);
  let diagnostics = $state<Diagnostic[]>([]);
  let busy = $state(false);
  let message = $state("");
  /** Last PDF written, revealed when the flash message is clicked. */
  let lastPdf = $state<string | null>(null);
  let loadError = $state("");
  /** The file changed on disk while there were unsaved edits. */
  let stale = $state(false);

  /** Plain copy of the reactive value for IPC. */
  const plain = (): Json => JSON.parse(JSON.stringify(value));

  const render = debounce(async (v: Json) => {
    busy = true;
    try {
      const out = await api.previewResume(id, v);
      // Keep the last good render visible while there are errors.
      if (out.svg.length) svg = out.svg;
      diagnostics = out.diagnostics;
    } catch (e) {
      diagnostics = [{ severity: "error", message: String(e), hints: [], file: null, line: null, column: null }];
    } finally {
      busy = false;
    }
  }, 250);

  function load() {
    const current = id;
    loadError = "";
    return Promise.all([api.getResume(current), api.getSchema(templateId)])
      .then(([v, s]) => {
        if (current !== id) return;
        value = v;
        schema = s;
        dirty = false;
        stale = false;
        errors = [];
        render(plain());
      })
      .catch((e) => (loadError = String(e)));
  }

  $effect(() => {
    load();
  });

  // Edits from outside the app (the agent, an editor, git) show up live.
  const unlisten = api.onWorkspaceChanged((paths) => {
    if (paths.includes(`resumes/${id}.yaml`)) {
      if (dirty) stale = true;
      else load();
    } else if (paths.some((p) => p === "profile.yaml" || p.startsWith(`templates/${templateId}/`))) {
      render(plain());
    }
  });
  onDestroy(() => unlisten.then((f) => f()));

  function update(v: Json | undefined) {
    value = (v ?? {}) as Record<string, Json>;
    dirty = true;
    render(plain());
  }

  async function persist() {
    errors = await api.saveResume(id, plain());
    if (errors.length === 0) {
      dirty = false;
      flash(t("common.saved"));
    }
  }

  async function savePdf() {
    const [diags, path] = await api.savePdf(id, plain());
    pdfDone(diags, path);
  }

  async function exportPdf() {
    const path = await save({ defaultPath: `${id}.pdf`, filters: [{ name: "PDF", extensions: ["pdf"] }] });
    if (!path) return;
    pdfDone(await api.exportPdf(id, plain(), path), path);
  }

  function pdfDone(diags: Diagnostic[], path: string | null) {
    const failed = !path || diags.some((d) => d.severity === "error");
    lastPdf = failed ? null : path;
    flash(failed ? t("resume.exportFailed") : t("resume.exported", { path: path! }));
  }

  function rename() {
    if (dirty) flash(t("resume.renameDirty"));
    else onrename();
  }

  async function remove() {
    if (!confirm(t("resume.confirmDelete", { id }))) return;
    await api.deleteResume(id);
    ondelete();
  }

  function flash(text: string) {
    message = text;
    setTimeout(() => (message = ""), 3000);
  }

  /** True while the panes are too narrow to sit side by side. */
  let narrow = $state(false);

  function onkeydown(e: KeyboardEvent) {
    if (isShortcut(e, "j")) {
      e.preventDefault();
      layout.toggleAgent();
      return;
    }
    if (isShortcut(e, "\\")) {
      e.preventDefault();
      layout.togglePreview("resume");
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      persist();
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="editor">
  <header class="topbar">
    <div class="crumbs">
      <span class="chip muted">▦ {template}</span>
      <span class="chip">▤ {id}{dirty ? " •" : ""}</span>
    </div>
    <div class="actions">
      <ViewSwitch id="resume" {narrow} />
      {#if message}
        {#if lastPdf}
          <button class="ghost flash" onclick={() => revealItemInDir(lastPdf!)}>{message}</button>
        {:else}<span class="flash">{message}</span>{/if}
      {/if}
      <button class="ghost" onclick={rename}>{t("common.rename")}</button>
      <button class="ghost danger" onclick={remove}>{t("common.delete")}</button>
      <button class:active={layout.agent} onclick={() => layout.toggleAgent()} title={t("agent.toggle")}>✦ {t("agent.title")}</button>
      <button onclick={savePdf} title={t("resume.savePdfHint")}>{t("resume.savePdf")}</button>
      <button onclick={exportPdf} title={t("resume.exportPdfHint")}>{t("resume.exportPdf")}</button>
      <button class="primary" onclick={persist} disabled={!dirty}>{t("common.save")}</button>
    </div>
  </header>

<div class="body">
<div class="panes">
<SplitPane id="resume" bind:narrow>
  {#snippet left()}
  <section class="form-pane">
    {#if loadError}
      <p class="error">{loadError}</p>
    {/if}
    {#if stale}
      <p class="stale">{t("agent.changedOnDisk")} <button class="small" onclick={load}>{t("agent.reload")}</button></p>
    {/if}
    {#if errors.length}
      <ul class="schema-errors">
        {#each errors as e, i (i)}<li><code>{e.path || "/"}</code> {e.message}</li>{/each}
      </ul>
    {/if}

    <div class="form-scroll">
      {#if schema}
        <ResumeForm {schema} {value} onchange={update} />
      {/if}
    </div>
  </section>
  {/snippet}

  {#snippet right()}
  <section class="preview-pane">
    <Preview {svg} {diagnostics} {busy} />
  </section>
  {/snippet}
</SplitPane>
</div>
{#if layout.agent}
  <AgentPane target={{ kind: "resume", id }} />
{/if}
</div>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .panes {
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .stale {
    margin: 0;
    padding: 0.5rem 1rem;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    background: color-mix(in srgb, var(--warn) 15%, var(--surface));
    font-size: 0.85rem;
  }
  button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .form-pane {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .form-scroll {
    flex: 1;
    overflow: auto;
    /* Lets the form switch to one column when this pane is narrow (see Field/ResumeForm). */
    container-type: inline-size;
    padding: 1.25rem 1.5rem 3rem;
  }
  .schema-errors {
    margin: 0;
    padding: 0.6rem 1rem 0.6rem 2rem;
    background: var(--danger-bg);
    color: var(--danger);
    font-size: 0.85rem;
  }
  .error {
    color: var(--danger);
    padding: 0 1rem;
  }
  .preview-pane {
    height: 100%;
    min-height: 0;
  }
</style>
