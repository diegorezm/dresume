import { invoke } from "@tauri-apps/api/core";
import { RangeSetBuilder, StateEffect, StateField, type Extension } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, type DecorationSet, type ViewUpdate } from "@codemirror/view";

interface Token {
  from: number;
  to: number;
  tag: string;
}

const setTokens = StateEffect.define<DecorationSet>();

// Decorations survive edits by being mapped through changes until fresh tokens arrive.
const tokensField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(deco, tr) {
    deco = deco.map(tr.changes);
    for (const e of tr.effects) if (e.is(setTokens)) deco = e.value;
    return deco;
  },
  provide: (f) => EditorView.decorations.from(f),
});

const marks = new Map<string, Decoration>();
const mark = (tag: string) => {
  let m = marks.get(tag);
  if (!m) marks.set(tag, (m = Decoration.mark({ class: tag })));
  return m;
};

/** Asks the backend (typst-syntax) for tokens after each pause in typing. */
const fetcher = ViewPlugin.fromClass(
  class {
    timer: ReturnType<typeof setTimeout> | undefined;
    constructor(readonly view: EditorView) {
      this.schedule(0);
    }
    update(u: ViewUpdate) {
      if (u.docChanged) this.schedule(60);
    }
    schedule(ms: number) {
      clearTimeout(this.timer);
      this.timer = setTimeout(() => this.run(), ms);
    }
    async run() {
      const doc = this.view.state.doc;
      let tokens: Token[];
      try {
        tokens = await invoke<Token[]>("highlight_typst", { text: doc.toString() });
      } catch {
        return; // e.g. running in a plain browser without the backend
      }
      if (this.view.state.doc !== doc) return; // stale; a newer request is pending
      const builder = new RangeSetBuilder<Decoration>();
      // Nested ranges come parent-first in document order, which is what the builder needs.
      const sorted = tokens.sort((a, b) => a.from - b.from || b.to - a.to);
      for (const t of sorted) builder.add(t.from, t.to, mark(t.tag));
      this.view.dispatch({ effects: setTokens.of(builder.finish()) });
    }
    destroy() {
      clearTimeout(this.timer);
    }
  },
);

/** Colors use theme variables, so they follow the app theme. */
const colors = EditorView.baseTheme({
  ".typ-comment": { color: "var(--muted)", fontStyle: "italic" },
  ".typ-punct": { color: "var(--muted)" },
  ".typ-escape, .typ-math-op, .typ-op": { color: "color-mix(in srgb, var(--accent) 70%, var(--text))" },
  ".typ-strong": { fontWeight: "700" },
  ".typ-emph": { fontStyle: "italic" },
  ".typ-link": { textDecoration: "underline" },
  ".typ-raw": { color: "color-mix(in srgb, var(--warn) 80%, var(--text))" },
  ".typ-label, .typ-ref": { color: "color-mix(in srgb, var(--accent) 60%, #2a9d8f)" },
  ".typ-heading": { fontWeight: "700", color: "var(--accent)" },
  ".typ-marker, .typ-term": { color: "var(--accent)", fontWeight: "600" },
  ".typ-math-delim, .typ-math-group": { color: "var(--muted)" },
  ".typ-key": { color: "var(--accent)", fontWeight: "600" },
  ".typ-num": { color: "color-mix(in srgb, #b5547a 80%, var(--text))" },
  ".typ-str": { color: "color-mix(in srgb, #5f8f3a 85%, var(--text))" },
  ".typ-func": { color: "color-mix(in srgb, #3b6fe0 75%, var(--text))" },
  ".typ-pol": { color: "color-mix(in srgb, #8a5cc7 75%, var(--text))" },
  ".typ-error": { textDecoration: "underline wavy var(--danger)" },
});

export function typst(): Extension {
  return [tokensField, fetcher, colors];
}
