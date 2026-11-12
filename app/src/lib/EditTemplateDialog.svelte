<script lang="ts">
  import { t } from "./i18n.svelte";
  import { api, type TemplateInfo } from "./api";
  import Modal from "./Modal.svelte";

  interface Props {
    template: TemplateInfo;
    templates: TemplateInfo[];
    onsaved: (template: TemplateInfo) => void;
    onclose: () => void;
  }

  let { template, templates, onsaved, onclose }: Props = $props();

  // Initial values only; the dialog edits its own copy.
  // svelte-ignore state_referenced_locally
  let name = $state(template.name);
  // svelte-ignore state_referenced_locally
  let id = $state(template.id);
  // svelte-ignore state_referenced_locally
  let description = $state(template.description);
  let error = $state("");
  let busy = $state(false);

  const taken = $derived(id !== template.id && templates.some((tpl) => tpl.id === id));
  const valid = $derived(name.trim() !== "" && /^[A-Za-z0-9_-]+$/.test(id) && !taken);

  async function submit() {
    busy = true;
    try {
      onsaved(await api.updateTemplate(template.id, id, name.trim(), description.trim()));
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<Modal
  icon="▦"
  title={t("editTemplate.title")}
  subtitle={t("editTemplate.subtitle")}
  submitLabel={t("common.save")}
  canSubmit={valid}
  {busy}
  {error}
  onsubmit={submit}
  {onclose}
>
  <div class="fields">
    <label class="m-label">
      <span>{t("newTemplate.name")}</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={name} autofocus />
    </label>
    <label class="m-label">
      <span>{t("newTemplate.folder")}</span>
      <div class="m-id" class:bad={taken}>
        <code>templates/</code>
        <input bind:value={id} spellcheck="false" />
      </div>
      {#if taken}<small class="m-warn">{t("newTemplate.taken")}</small>{/if}
    </label>
    <label class="m-label wide">
      <span>{t("editTemplate.description")}</span>
      <input bind:value={description} />
    </label>
  </div>
  {#if id !== template.id && template.resumes.length}
    <small class="m-hint">{t("editTemplate.relink", { n: template.resumes.length })}</small>
  {/if}
</Modal>

<style>
  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }
  .wide {
    grid-column: 1 / -1;
  }
  .m-hint {
    display: block;
    margin-top: 0.75rem;
    color: var(--muted);
  }
</style>
