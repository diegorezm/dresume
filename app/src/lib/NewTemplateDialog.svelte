<script lang="ts">
  import { t } from "./i18n.svelte";
  import { api, slug, type TemplateInfo } from "./api";
  import Modal from "./Modal.svelte";

  interface Props {
    templates: TemplateInfo[];
    oncreated: (template: TemplateInfo) => void;
    onclose: () => void;
  }

  let { templates, oncreated, onclose }: Props = $props();

  type Source = { id: string; name: string; description: string; kind: "starter" | "blank" | "copy" };

  const sources: Source[] = $derived([
    { id: "@starter", name: "Basic", description: t("newTemplate.basicDesc"), kind: "starter" },
    { id: "", name: t("newTemplate.blank"), description: t("newTemplate.blankDesc"), kind: "blank" },
    ...templates.map((tpl) => ({
      id: tpl.id,
      name: tpl.name,
      description: tpl.description || t("newTemplate.copyOf", { id: tpl.id }),
      kind: "copy" as const,
    })),
  ]);

  let name = $state("");
  let id = $state("");
  let idTouched = $state(false);
  let from = $state("@starter");
  let error = $state("");
  let busy = $state(false);

  // The id follows the name until the user edits it.
  const effectiveId = $derived(idTouched ? id : slug(name));
  const taken = $derived(templates.some((tpl) => tpl.id === effectiveId));
  const valid = $derived(name.trim() !== "" && /^[A-Za-z0-9_-]+$/.test(effectiveId) && !taken);

  async function submit() {
    busy = true;
    try {
      oncreated(await api.newTemplate(effectiveId, name.trim(), from || null));
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<Modal
  icon="▦"
  title={t("newTemplate.title")}
  subtitle={t("newTemplate.subtitle")}
  submitLabel={t("newTemplate.submit")}
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
      <input bind:value={name} placeholder={t("newTemplate.namePlaceholder")} autofocus />
    </label>
    <label class="m-label">
      <span>{t("newTemplate.folder")}</span>
      <div class="m-id" class:bad={taken}>
        <code>templates/</code>
        <input
          value={effectiveId}
          oninput={(e) => { idTouched = true; id = e.currentTarget.value; }}
          placeholder="modern"
          spellcheck="false"
        />
      </div>
      {#if taken}<small class="m-warn">{t("newTemplate.taken")}</small>{/if}
    </label>
  </div>

  <div class="m-section">{t("newTemplate.startFrom")}</div>
  <div class="m-options">
    {#each sources as s (s.id)}
      <button type="button" class="m-option" class:active={from === s.id} onclick={() => (from = s.id)}>
        {#if s.kind === "blank"}
          <div class="m-thumb dashed" aria-hidden="true">＋</div>
        {:else}
          <div class="m-thumb" class:accent={s.kind === "starter"} aria-hidden="true">
            <i class="t1"></i><i class="t2"></i><i class="r"></i><i></i><i class="s"></i><i></i><i class="r"></i><i class="s"></i>
          </div>
        {/if}
        <div class="m-option-text">
          <strong>{s.name}</strong>
          <small>{s.description}</small>
        </div>
        {#if from === s.id}<span class="m-check">✓</span>{/if}
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
</style>
