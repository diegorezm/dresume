<script lang="ts">
  import { t } from "./i18n.svelte";
  import { api, type ResumeInfo } from "./api";
  import Modal from "./Modal.svelte";

  interface Props {
    resume: string;
    resumes: ResumeInfo[];
    onsaved: (resume: ResumeInfo) => void;
    onclose: () => void;
  }

  let { resume, resumes, onsaved, onclose }: Props = $props();

  // svelte-ignore state_referenced_locally
  let id = $state(resume);
  let error = $state("");
  let busy = $state(false);

  const taken = $derived(id !== resume && resumes.some((r) => r.id === id));
  const valid = $derived(/^[A-Za-z0-9_-]+$/.test(id) && id !== resume && !taken);

  async function submit() {
    busy = true;
    try {
      onsaved(await api.renameResume(resume, id));
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<Modal
  icon="▤"
  title={t("renameResume.title")}
  subtitle={t("renameResume.subtitle")}
  submitLabel={t("common.save")}
  canSubmit={valid}
  {busy}
  {error}
  onsubmit={submit}
  {onclose}
>
  <label class="m-label">
    <span>{t("renameResume.file")}</span>
    <div class="m-id" class:bad={taken}>
      <code>resumes/</code>
      <!-- svelte-ignore a11y_autofocus -->
      <input bind:value={id} spellcheck="false" autofocus />
      <code>.yaml</code>
    </div>
    {#if taken}<small class="m-warn">{t("renameResume.taken")}</small>{/if}
  </label>
</Modal>
