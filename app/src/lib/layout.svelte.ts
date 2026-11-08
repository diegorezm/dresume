// Per-user layout preferences (sidebar, which panes each editor shows), kept in localStorage.

/** What a two-pane editor shows: both panes, only the editor, or only the preview. */
export type PaneMode = "split" | "left" | "right";

const KEY = "dresume.layout";

interface Saved {
  sidebar: boolean;
  agent: boolean;
  panes: Record<string, PaneMode>;
}

class Layout {
  /** Whether the sidebar is visible. */
  sidebar = $state(true);
  /** Whether the AI agent panel is open in the resume editor. */
  agent = $state(false);
  panes = $state<Record<string, PaneMode>>({});

  constructor() {
    try {
      const saved = JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<Saved>;
      if (typeof saved.sidebar === "boolean") this.sidebar = saved.sidebar;
      if (typeof saved.agent === "boolean") this.agent = saved.agent;
      if (saved.panes && typeof saved.panes === "object") this.panes = saved.panes;
    } catch {}
  }

  mode(id: string): PaneMode {
    return this.panes[id] ?? "split";
  }

  setMode(id: string, mode: PaneMode) {
    this.panes = { ...this.panes, [id]: mode };
    this.save();
  }

  /** Shows/hides the preview (right pane) of an editor. */
  togglePreview(id: string) {
    this.setMode(id, this.mode(id) === "left" ? "split" : "left");
  }

  toggleSidebar() {
    this.sidebar = !this.sidebar;
    this.save();
  }

  toggleAgent() {
    this.agent = !this.agent;
    this.save();
  }

  private save() {
    try {
      localStorage.setItem(KEY, JSON.stringify({ sidebar: this.sidebar, agent: this.agent, panes: this.panes }));
    } catch {}
  }
}

export const layout = new Layout();

/** True for Ctrl (or ⌘ on macOS) + key, without other modifiers. */
export function isShortcut(e: KeyboardEvent, key: string) {
  return (e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === key.toLowerCase();
}
