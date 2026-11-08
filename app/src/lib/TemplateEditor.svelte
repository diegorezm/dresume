<script lang="ts">
  import { t } from "./i18n.svelte";
  import { api, debounce, type Diagnostic, type TemplateInfo } from "./api";
  import CodeEditor from "./CodeEditor.svelte";
  import Preview from "./Preview.svelte";
  import SplitPane from "./SplitPane.svelte";
  import ViewSwitch from "./ViewSwitch.svelte";
  import { isShortcut, layout } from "./layout.svelte";

  interface Props {
    template: TemplateInfo;
  }

  let { template }: Props = $props();

  let files = $state<string[]>([]);
  let contents = $state<Record<string, string>>({});
  let saved = $state<Record<string, string>>({});
  let active = $state("template.typ");
  let svg = $state<string[]>([]);
  let diagnostics = $state<Diagnostic[]>([]);
  let busy = $state(false);
  let message = $state("");
  let editor: CodeEditor | undefined = $state();

  const dirtyFiles = $derived(files.filter((f) => contents[f] !== saved[f]));

  const render = debounce(async () => {
    busy = true;
    try {
      const out = await api.previewTemplate(template.id, $state.snapshot(contents));
      if (out.svg.length) svg = out.svg;
      diagnostics = out.diagnostics;
    } catch (e) {
      diagnostics = [{ severity: "error", message: String(e), hints: [], file: null, line: null, column: null }];
    } finally {
      busy = false;
    }
  }, 300);

  $effect(() => {
    const id = template.id;
    (async () => {
      const names = await api.templateFiles(id);
      const texts = await Promise.all(names.map((n) => api.readTemplateFile(id, n)));
      if (id !== template.id) return;
      files = names;
      contents = Object.fromEntries(names.map((n, i) => [n, texts[i]]));
      saved = { ...contents };
      active = names[0] ?? "template.typ";
      render();
    })();
  });

  function edit(text: string) {
    contents[active] = text;
    render();
  }

  async function persist() {
    for (const f of dirtyFiles) {
      await api.writeTemplateFile(template.id, f, contents[f]);
      saved[f] = contents[f];
    }
    message = t("template.saved", { n: template.resumes.length });
    setTimeout(() => (message = ""), 3000);
  }

  function jump(d: Diagnostic) {
    const prefix = `templates/${template.id}/`;
    if (!d.file?.startsWith(prefix)) return;
    const name = d.file.slice(prefix.length);
    if (!files.includes(name)) return;
    active = name;
    // Wait for the editor to swap documents before moving the cursor.
    setTimeout(() => editor?.goto(d.line ?? 1, d.column ?? 1));
  }

  /** True while the panes are too narrow to sit side by side. */
  let narrow = $state(false);

  function onkeydown(e: KeyboardEvent) {
    if (isShortcut(e, "\\")) {
      e.preventDefault();
      layout.togglePreview("template");
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
      <span class="chip">▦ {template.name}</span>
      <span class="chip muted" title={t("template.previewHint")}>
        {t("template.usedBy", { n: template.resumes.length })}
      </span>
    </div>
    <div class="actions">
      <ViewSwitch id="template" {narrow} />
      {#if message}<span class="flash">{message}</span>{/if}
      <button class="primary" onclick={persist} disabled={dirtyFiles.length === 0}>{t("common.save")}</button>
    </div>
  </header>

<div class="panes">
<SplitPane id="template" bind:narrow>
  {#snippet left()}
  <section class="code-pane">
    <nav class="tabs">
      {#each files as f (f)}
        <button class="tab" class:active={f === active} onclick={() => (active = f)}>
          {f}{contents[f] !== saved[f] ? " •" : ""}
        </button>
      {/each}
    </nav>
    <div class="code">
      {#if files.includes(active)}
        <CodeEditor bind:this={editor} name={active} value={contents[active]} onchange={edit} />
      {/if}
    </div>
  </section>
  {/snippet}

  {#snippet right()}
  <section class="preview-pane">
    <Preview {svg} {diagnostics} {busy} ondiagnostic={jump} />
  </section>
  {/snippet}
</SplitPane>
</div>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .panes {
    flex: 1;
    min-height: 0;
  }
  .code-pane {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
  }
  .tab {
    border: none;
    border-radius: 0;
    background: transparent;
    color: var(--muted);
    padding: 0.5rem 0.9rem;
    font-family: var(--mono);
    font-size: 0.8rem;
    border-bottom: 2px solid transparent;
  }
  .tab.active {
    border-bottom-color: var(--accent);
    color: var(--accent);
  }
  .code {
    flex: 1;
    min-height: 0;
  }
  .preview-pane {
    height: 100%;
    min-height: 0;
  }
</style>
