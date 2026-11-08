<script lang="ts">
  import { t } from "./i18n.svelte";
  import { layout } from "./layout.svelte";
  import type { AgentTarget } from "./api";
  import AgentPanel from "./AgentPanel.svelte";

  let { target }: { target: AgentTarget } = $props();

  /** Drags the panel's left edge; the width is kept between sessions. */
  function startResize(e: PointerEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startW = layout.agentWidth;
    const max = Math.max(320, window.innerWidth * 0.6);
    const move = (ev: PointerEvent) => {
      layout.agentWidth = Math.min(max, Math.max(280, startW + startX - ev.clientX));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      document.body.style.cursor = "";
      layout.setAgentWidth(layout.agentWidth);
    };
    document.body.style.cursor = "col-resize";
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }
</script>

<div class="agent-pane" style:width="{layout.agentWidth}px">
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="agent-resize"
    role="separator"
    aria-orientation="vertical"
    title={t("common.resize")}
    onpointerdown={startResize}
    ondblclick={() => layout.setAgentWidth(360)}
  ></div>
  <AgentPanel {target} />
</div>

<style>
  .agent-pane {
    position: relative;
    flex: none;
    min-width: 280px;
    max-width: 60%;
    min-height: 0;
  }
  .agent-resize {
    position: absolute;
    left: -3px;
    top: 0;
    bottom: 0;
    width: 6px;
    cursor: col-resize;
    z-index: 5;
  }
  .agent-resize:hover {
    background: color-mix(in srgb, var(--accent) 35%, transparent);
  }
</style>
