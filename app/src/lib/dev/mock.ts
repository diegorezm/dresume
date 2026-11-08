// Browser-only stand-in for the Tauri backend, used by `pnpm dev` outside the app.
// Data comes from fixture.json (run `node scripts/dev-fixture.mjs`). Writes are kept in memory.
import { mockIPC } from "@tauri-apps/api/mocks";

export async function installMock() {
  const fixture = (await import("./fixture.json")).default as any;
  let settings: Record<string, unknown> = { lastWorkspace: fixture.summary.root };
  const hidden: Record<string, string[]> = {};

  mockIPC((cmd, args: any) => {
    switch (cmd) {
      case "load_settings": return { settings, themes: [] };
      case "save_settings": settings = args.settings; return null;
      case "open_workspace":
      case "workspace_summary": return fixture.summary;
      case "get_resume": return fixture.resumes[args.id];
      case "get_schema": return fixture.schemas[args.template];
      case "get_profile": return fixture.profile;
      case "profile_visibility": return {
        fields: ["name", "title", "email", "phone", "location", "website", "github", "linkedin"],
        templates: fixture.summary.templates.map((t: any) => ({ id: t.id, name: t.name, hidden: hidden[t.id] ?? [] })),
      };
      case "set_profile_hidden": hidden[args.template] = args.hidden; return null;
      case "save_resume": fixture.resumes[args.id] = args.value; return [];
      case "preview_resume": return { svg: fixture.svg[args.id] ?? [], diagnostics: fixture.diagnostics[args.id] ?? [] };
      case "template_files": {
        const names = Object.keys(fixture.templateFiles[args.template] ?? {});
        return names.sort((a, b) => Number(b === "template.typ") - Number(a === "template.typ") || a.localeCompare(b));
      }
      case "read_template_file": return fixture.templateFiles[args.template]?.[args.name] ?? "";
      case "preview_template": {
        const t = fixture.summary.templates.find((x: any) => x.id === args.template);
        const id = t?.resumes[0];
        return { svg: id ? fixture.svg[id] : [], diagnostics: [] };
      }
      case "highlight_typst": return roughHighlight(args.text);
      case "plugin:dialog|open":
      case "plugin:dialog|save": return null;
      default: return null;
    }
  });
}

/** Regex approximation of the backend's typst-syntax highlighter, good enough for a browser preview. */
function roughHighlight(text: string) {
  const rules: [RegExp, string][] = [
    [/\/\/[^\n]*/g, "typ-comment"],
    [/"[^"\n]*"/g, "typ-str"],
    [/#(?:let|set|show|import|include|if|else|for|in|return)\b/g, "typ-key"],
    [/\b(?:let|set|show|if|else|for|in|and|or|not|none|auto|true|false)\b/g, "typ-key"],
    [/#?[a-z][\w-]*(?=\()/g, "typ-func"],
    [/^=+ [^\n]*/gm, "typ-heading"],
    [/\b\d+(?:\.\d+)?(?:pt|em|cm|mm|%|fr|deg)?\b/g, "typ-num"],
  ];
  const tokens: { from: number; to: number; tag: string }[] = [];
  const taken = new Uint8Array(text.length);
  for (const [re, tag] of rules) {
    for (const m of text.matchAll(re)) {
      const from = m.index!;
      const to = from + m[0].length;
      if (taken.subarray(from, to).some(Boolean)) continue;
      taken.fill(1, from, to);
      tokens.push({ from, to, tag });
    }
  }
  return tokens;
}
