<script lang="ts">
  import { t } from "./i18n.svelte";
  import { api, slug, type ResumeInfo, type TemplateInfo } from "./api";
  import Modal from "./Modal.svelte";

  interface Props {
    templates: TemplateInfo[];
    resumes: ResumeInfo[];
    /** Template selected when the dialog opens. */
    template?: string;
    oncreated: (resume: ResumeInfo) => void;
    onclose: () => void;
  }

  let { templates, resumes, template: initialTemplate, oncreated, onclose }: Props = $props();

  let name = $state("");
  let id = $state("");
  let idTouched = $state(false);
  // svelte-ignore state_referenced_locally
  let template = $state(initialTemplate ?? templates[0]?.id ?? "");
  /** "" = the template's sample content, otherwise a resume id to copy. */
  let from = $state("");
  let error = $state("");
  let busy = $state(false);

  // Only resumes of the same template can be copied: their content fits its schema.
  const siblings = $derived(resumes.filter((r) => r.template === template));
  const effectiveId = $derived(idTouched ? id : slug(name));
  const taken = $derived(resumes.some((r) => r.id === effectiveId));
  const valid = $derived(template !== "" && /^[A-Za-z0-9_-]+$/.test(effectiveId) && !taken);

  function pickTemplate(t: string) {
    template = t;
    from = "";
  }

  async function submit() {
    busy = true;
    try {
      oncreated(from ? await api.duplicateResume(effectiveId, from) : await api.newResume(effectiveId, template));
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<Modal
  icon="▤"
  title={t("newResume.title")}
  subtitle={t("newResume.subtitle")}
  submitLabel={t("newResume.submit")}
  canSubmit={valid}
  {busy}
  {error}
  onsubmit={submit}
  {onclose}
>
  <div class="fields">
    <label class="m-label">
      <span>{t("newResume.name")}</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={name} placeholder={t("newResume.namePlaceholder")} autofocus />
    </label>
    <label class="m-label">
      <span>{t("newResume.file")}</span>
      <div class="m-id" class:bad={taken}>
        <code>resumes/</code>
        <input
          value={effectiveId}
          oninput={(e) => { idTouched = true; id = e.currentTarget.value; }}
          placeholder="backend-en"
          spellcheck="false"
        />
        <code class="ext">.yaml</code>
      </div>
      {#if taken}<small class="m-warn">{t("newResume.taken")}</small>{/if}
    </label>
  </div>

  <div class="m-section">{t("newResume.template")}</div>
  <div class="m-options">
    {#each templates as tpl (tpl.id)}
      <button type="button" class="m-option" class:active={template === tpl.id} onclick={() => pickTemplate(tpl.id)}>
        <div class="m-thumb" aria-hidden="true">
          <i class="t1"></i><i class="t2"></i><i class="r"></i><i></i><i class="s"></i><i></i><i class="r"></i><i class="s"></i>
        </div>
        <div class="m-option-text">
          <strong>{tpl.name}</strong>
          <small>{t("home.resumes", { n: tpl.resumes.length })}{tpl.description ? ` · ${tpl.description}` : ""}</small>
        </div>
        {#if template === tpl.id}<span class="m-check">✓</span>{/if}
      </button>
    {/each}
  </div>

  <div class="m-section">{t("newResume.content")}</div>
  <div class="content">
    <button type="button" class="pick" class:active={from === ""} onclick={() => (from = "")}>
      <span class="dot"></span>
      <span><strong>{t("newResume.sample")}</strong><small>{t("newResume.sampleDesc")}</small></span>
    </button>
    {#each siblings as r (r.id)}
      <button type="button" class="pick" class:active={from === r.id} onclick={() => (from = r.id)}>
        <span class="dot"></span>
        <span><strong>{t("newResume.copyOf", { id: r.id })}</strong><small>{t("newResume.copyDesc")}</small></span>
      </button>
    {/each}
  </div>
</Modal>

<style>
  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }
  .ext {
    padding-right: 0.65rem;
  }
  .content {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    max-height: 180px;
    overflow: auto;
    padding: 2px;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    text-align: left;
    border-radius: var(--radius-sm);
    border: 1.5px solid var(--border);
    background: var(--bg);
    padding: 0.55rem 0.8rem;
  }
  .pick:hover:not(:disabled) {
    background: var(--bg);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  }
  .pick.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .pick > span:last-child {
    display: flex;
    flex-direction: column;
  }
  .pick small {
    color: var(--muted);
    font-size: 0.8rem;
  }
  .dot {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1.5px solid var(--border);
    flex-shrink: 0;
    transition: border 0.15s;
  }
  .pick.active .dot {
    border: 5px solid var(--accent);
  }
</style>
