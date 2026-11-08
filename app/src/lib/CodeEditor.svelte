<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState, type Extension } from "@codemirror/state";
  import { json } from "@codemirror/lang-json";
  import { yaml } from "@codemirror/lang-yaml";
  import { syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";
  import { oneDarkHighlightStyle } from "@codemirror/theme-one-dark";
  import { appearance } from "./theme.svelte";
  import { typst } from "./typstHighlight";

  interface Props {
    /** File name, used to pick the language mode. */
    name: string;
    value: string;
    onchange: (value: string) => void;
  }

  let { name, value, onchange }: Props = $props();
  let host: HTMLDivElement;
  let view: EditorView | undefined;

  function language(file: string): Extension[] {
    if (file.endsWith(".json")) return [json()];
    if (file.endsWith(".yaml") || file.endsWith(".yml")) return [yaml()];
    if (file.endsWith(".typ")) return [typst()];
    return [];
  }

  // Colors come from the app theme's CSS variables, so switching themes needs no rebuild.
  const chrome = EditorView.theme({
    "&": { height: "100%", backgroundColor: "var(--surface)", color: "var(--text)" },
    ".cm-scroller": { fontFamily: "var(--mono)", lineHeight: "1.6" },
    ".cm-content": { caretColor: "var(--accent)", padding: "0.75rem 0" },
    ".cm-cursor": { borderLeftColor: "var(--accent)" },
    ".cm-gutters": { backgroundColor: "var(--surface)", color: "var(--muted)", border: "none" },
    ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "color-mix(in srgb, var(--accent) 6%, transparent)" },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
      backgroundColor: "color-mix(in srgb, var(--accent) 22%, transparent) !important",
    },
    ".cm-matchingBracket": { backgroundColor: "var(--accent-soft)", outline: "none" },
    "&.cm-focused": { outline: "none" },
  });

  function makeState(doc: string, file: string, dark: boolean) {
    return EditorState.create({
      doc,
      extensions: [
        basicSetup,
        ...language(file),
        chrome,
        EditorView.darkTheme.of(dark),
        syntaxHighlighting(dark ? oneDarkHighlightStyle : defaultHighlightStyle),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) onchange(u.state.doc.toString());
        }),
      ],
    });
  }

  onMount(() => {
    view = new EditorView({ state: makeState(value, name, appearance.dark), parent: host });
    return () => view?.destroy();
  });

  // Swap documents when the file changes; external value updates replace the text.
  let currentName = untrack(() => name);
  let currentDark = untrack(() => appearance.dark);
  $effect(() => {
    if (!view) return;
    const dark = appearance.dark;
    if (name !== currentName || dark !== currentDark) {
      currentName = name;
      currentDark = dark;
      view.setState(makeState(value, name, dark));
    } else if (value.replace(/\r\n?/g, "\n") !== view.state.doc.toString()) {
      // CodeMirror stores LF; comparing normalized text keeps CRLF files from looking edited.
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: value } });
    }
  });

  /** Moves the cursor to a 1-based line/column and focuses the editor. */
  export function goto(line: number, column = 1) {
    if (!view) return;
    const l = view.state.doc.line(Math.min(line, view.state.doc.lines));
    const pos = Math.min(l.from + column - 1, l.to);
    view.dispatch({ selection: { anchor: pos }, scrollIntoView: true });
    view.focus();
  }
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }
</style>
