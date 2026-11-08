<script lang="ts">
  import { t } from "./i18n.svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { api, debounce, type Diagnostic, type Json, type Schema, type SchemaError } from "./api";
  import ResumeForm from "./form/ResumeForm.svelte";
  import Preview from "./Preview.svelte";

  interface Props {
    id: string;
    /** Template display name. */
    template: string;
    templateId: string;
    ondelete: () => void;
  }

  let { id, template, templateId, ondelete }: Props = $props();

  let schema = $state<Schema | null>(null);
  let value = $state<Record<string, Json>>({});
  let dirty = $state(false);
  let errors = $state<SchemaError[]>([]);
  let svg = $state<string[]>([]);
  let diagnostics = $state<Diagnostic[]>([]);
  let busy = $state(false);
  let message = $state("");
  let loadError = $state("");

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

  $effect(() => {
    const current = id;
    loadError = "";
    Promise.all([api.getResume(current), api.getSchema(templateId)])
      .then(([v, s]) => {
        if (current !== id) return;
        value = v;
        schema = s;
        dirty = false;
        errors = [];
        render(plain());
      })
      .catch((e) => (loadError = String(e)));
  });

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

  async function exportPdf() {
    const path = await save({ defaultPath: `${id}.pdf`, filters: [{ name: "PDF", extensions: ["pdf"] }] });
    if (!path) return;
    const diags = await api.exportPdf(id, plain(), path);
    flash(diags.some((d) => d.severity === "error") ? t("resume.exportFailed") : t("resume.exported", { path }));
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

  function onkeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      persist();
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="split">
  <section class="form-pane">
    <header class="topbar">
      <div class="crumbs">
        <span class="chip muted">▦ {template}</span>
        <span class="chip">▤ {id}{dirty ? " •" : ""}</span>
      </div>
      <div class="actions">
        {#if message}<span class="flash">{message}</span>{/if}
        <button class="ghost danger" onclick={remove}>{t("common.delete")}</button>
        <button onclick={exportPdf}>{t("resume.exportPdf")}</button>
        <button class="primary" onclick={persist} disabled={!dirty}>{t("common.save")}</button>
      </div>
    </header>

    {#if loadError}
      <p class="error">{loadError}</p>
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

  <section class="preview-pane">
    <Preview {svg} {diagnostics} {busy} />
  </section>
</div>

<style>
  .split {
    display: grid;
    grid-template-columns: minmax(380px, 1fr) minmax(420px, 1fr);
    height: 100%;
    min-height: 0;
  }
  .form-pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
  }
  .form-scroll {
    flex: 1;
    overflow: auto;
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
    min-height: 0;
  }
</style>
