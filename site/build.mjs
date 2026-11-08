// Builds the dresume website (landing + docs, en/pt) into site/dist/. No dependencies: `node site/build.mjs`.
// Reference sections (CLI, settings, themes, shortcuts) are generated from the app and CLI sources,
// so the docs follow the code. The build fails when a keyboard shortcut in the app is undocumented.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const site = path.dirname(fileURLToPath(import.meta.url));
const root = path.dirname(site);
const out = path.join(site, "dist");
const read = (p) => fs.readFileSync(path.join(root, p), "utf8").replace(/\r/g, "");

const LANGS = ["en", "pt"];
const SITE_URL = "https://diegorezm.github.io/dresume/";
const pagePath = (lang, page) => `${lang === "en" ? "" : `${lang}/`}${page === "docs" ? "docs/" : ""}`;
const strings = JSON.parse(read("site/i18n.json"));

// ---------- extract data from the sources ----------

/** Evaluates a JS literal that follows `marker` in `src` (array or object). */
function literal(src, marker) {
  const start = src.indexOf(marker);
  if (start < 0) throw new Error(`not found: ${marker}`);
  const eq = src.indexOf("=", start);
  const open = eq + src.slice(eq).search(/[[{]/);
  let depth = 0;
  for (let i = open; i < src.length; i++) {
    if ("[{".includes(src[i])) depth++;
    else if ("]}".includes(src[i]) && --depth === 0) return new Function(`return ${src.slice(open, i + 1)}`)();
  }
  throw new Error(`unterminated: ${marker}`);
}

const themeSrc = read("app/src/lib/theme.svelte.ts");
const THEMES = literal(themeSrc, "BUILTIN_THEMES");
const DEFAULTS = literal(themeSrc, "DEFAULT_SETTINGS");

/** App UI strings per locale: plain strings, and template-literal functions with `${p.x}` turned into "…". */
function appStrings() {
  const src = read("app/src/lib/i18n.svelte.ts");
  const split = src.indexOf("\nconst pt");
  const parse = (part) => {
    const dict = {};
    for (const m of part.matchAll(/"([\w.]+)":\s*(?:"((?:[^"\\]|\\.)*)"|\(p\) => `([^`]*)`)/g))
      dict[m[1]] = m[2] ?? m[3].replace(/\$\{[^}]+\}/g, "…");
    return dict;
  };
  const locales = literal(src, "LOCALES");
  return { en: parse(src.slice(0, split)), pt: parse(src.slice(split)), locales };
}
const APP = appStrings();

/** CLI commands from the clap `Command` enum and its doc comments. */
function cliCommands() {
  const src = read("crates/cli/src/main.rs");
  const body = src.slice(src.indexOf("enum Command {"), src.indexOf("\n}\n", src.indexOf("enum Command {")));
  const cmds = [];
  let doc = [];
  const lines = body.split("\n").slice(1);
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const d = line.match(/^    \/\/\/ ?(.*)/);
    if (d) { doc.push(d[1]); continue; }
    const v = line.match(/^    (\w+)(.*)/);
    if (!v) continue;
    let fields = v[2];
    if (fields.trim() === "{") while (!lines[++i].startsWith("    }")) fields += "\n" + lines[i];
    const kebab = v[1].replace(/[a-z][A-Z]/g, (s) => `${s[0]}-${s[1]}`).toLowerCase();
    cmds.push({ name: kebab, doc: doc.join(" "), args: parseFields(fields) });
    doc = [];
  }
  return cmds;
}

function parseFields(text) {
  const args = [];
  const re = /(?:\/\/\/ ?(.*)\n\s*)?(?:#\[arg\(([^)]*)\)\]\s*)?(\w+): ([\w<>]+)/g;
  for (const m of text.matchAll(re)) {
    const [, doc, attr, name, type] = m;
    const flags = attr ? attr.split(",").map((s) => s.trim()) : [];
    const def = flags.find((f) => f.startsWith("default_value"))?.split("=")[1].trim().replace(/"/g, "");
    const optional = type.startsWith("Option") || type.startsWith("Vec") || def !== undefined;
    let usage;
    if (!attr) usage = type.startsWith("Vec") ? `${name}…` : `<${name}>`;
    else usage = [flags.includes("short") && `-${name[0]}`, flags.includes("long") && `--${name}`].filter(Boolean).join(", ") + ` <${name}>`;
    args.push({ usage: optional ? `[${usage}]` : usage, doc, def: def?.includes("::") ? def.split("::")[1].toLowerCase() : def });
  }
  return args;
}
const CLI = cliCommands();

/** Keys used with Ctrl/Cmd in the app's components. */
function shortcutKeys() {
  const keys = new Set();
  const walk = (dir) => {
    for (const f of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, f.name);
      if (f.isDirectory()) walk(p);
      else if (f.name.endsWith(".svelte")) {
        const src = fs.readFileSync(p, "utf8");
        for (const m of src.matchAll(/isShortcut\(e, "(.+?)"\)|metaKey\) && e\.key === "(.)"/g)) keys.add((m[1] ?? m[2]).replace("\\\\", "\\"));
      }
    }
  };
  walk(path.join(root, "app/src"));
  return [...keys];
}

// ---------- tiny markdown ----------

const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
const slug = (s) => s.toLowerCase().replace(/<[^>]+>/g, "").replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "");

function inline(s) {
  return s
    .split(/(`[^`]+`)/)
    .map((part) =>
      part.startsWith("`") && part.endsWith("`") && part.length > 1
        ? `<code>${esc(part.slice(1, -1))}</code>`
        : esc(part)
            .replace(/&lt;(\/?kbd)&gt;/g, "<$1>")
            .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
            .replace(/\*(.+?)\*/g, "<em>$1</em>")
            .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>'),
    )
    .join("");
}

function markdown(src) {
  const lines = src.split("\n");
  const toc = [];
  let html = "";
  for (let i = 0; i < lines.length; ) {
    const line = lines[i];
    let m;
    if (!line.trim()) { i++; continue; }
    if (line.startsWith("```")) {
      const lang = line.slice(3).trim();
      const code = [];
      while (++i < lines.length && !lines[i].startsWith("```")) code.push(lines[i]);
      i++;
      html += `<pre${lang ? ` data-lang="${lang}"` : ""}><code>${esc(code.join("\n"))}</code></pre>\n`;
    } else if ((m = line.match(/^(#{1,4}) (.*)/))) {
      // `## Title {#id}` sets the anchor, so both languages share it.
      const [, title, custom] = m[2].match(/^(.*?)(?: \{#([\w-]+)\})?$/);
      const level = m[1].length;
      const text = inline(title);
      const id = custom ?? slug(title);
      if (level === 2 || level === 3) toc.push({ level, id, text });
      html += `<h${level} id="${id}"><a class="anchor" href="#${id}">${text}</a></h${level}>\n`;
      i++;
    } else if (line.startsWith("|")) {
      const rows = [];
      while (i < lines.length && lines[i].startsWith("|")) rows.push(lines[i++]);
      const cells = (r) => r.replace(/^\||\|$/g, "").split("|").map((c) => inline(c.trim()));
      html += `<div class="table"><table><thead><tr>${cells(rows[0]).map((c) => `<th>${c}</th>`).join("")}</tr></thead><tbody>`;
      html += rows.slice(2).map((r) => `<tr>${cells(r).map((c) => `<td>${c}</td>`).join("")}</tr>`).join("");
      html += "</tbody></table></div>\n";
    } else if (/^(- |\d+\. )/.test(line)) {
      const ordered = /^\d/.test(line);
      const items = [];
      while (i < lines.length && /^(- |\d+\. |  )/.test(lines[i])) {
        if (lines[i].startsWith("  ")) items[items.length - 1] += " " + lines[i].trim();
        else items.push(lines[i].replace(/^(- |\d+\. )/, ""));
        i++;
      }
      const tag = ordered ? "ol" : "ul";
      html += `<${tag}>${items.map((x) => `<li>${inline(x)}</li>`).join("")}</${tag}>\n`;
    } else if (line.startsWith("> ")) {
      const quote = [];
      while (i < lines.length && lines[i].startsWith("> ")) quote.push(lines[i++].slice(2));
      html += `<blockquote>${inline(quote.join(" "))}</blockquote>\n`;
    } else if (/^\{\{\w+\}\}$/.test(line.trim())) {
      html += line.trim() + "\n";
      i++;
    } else {
      const para = [];
      while (i < lines.length && lines[i].trim() && !/^(#|```|\||- |\d+\. |> |\{\{)/.test(lines[i])) para.push(lines[i++]);
      html += `<p>${inline(para.join(" "))}</p>\n`;
    }
  }
  return { html, toc };
}

// ---------- generated sections ----------

function genCli() {
  return CLI.map((c) => {
    const usage = ["dresume", c.name, ...c.args.map((a) => a.usage)].join(" ");
    const opts = c.args.filter((a) => a.doc || a.def);
    return `<div class="cmd card"><pre><code>${esc(usage)}</code></pre><p>${inline(c.doc)}</p>${
      opts.length
        ? `<ul>${opts.map((a) => `<li><code>${esc(a.usage.replace(/^\[|\]$/g, ""))}</code> ${a.doc ? inline(a.doc) : ""}${a.def ? ` <span class="muted">(default: <code>${esc(a.def)}</code>)</span>` : ""}</li>`).join("")}</ul>`
        : ""
    }</div>`;
  }).join("\n");
}

function genSettings(lang) {
  const a = APP[lang];
  const s = strings[lang];
  const theme = THEMES.find((x) => x.id === DEFAULTS.theme);
  const rows = [
    [a["settings.language"], Object.values(APP.locales).join(", "), DEFAULTS.locale ?? s["docs.system"]],
    [a["settings.theme"], THEMES.map((x) => x.name).join(", "), theme.name],
    [a["settings.accent"], s["docs.anyColor"], `<span class="dot" style="background:${theme.colors.accent}"></span> ${s["docs.fromTheme"]}`],
    [a["settings.font"], `Figtree, Inter, ${a["settings.fontSystem"]}, ${a["settings.fontSerif"]}`, "Figtree"],
    [a["settings.corners"], ["sharp", "soft", "round"].map((r) => a[`settings.${r}`]).join(", "), a[`settings.${DEFAULTS.radius}`]],
    [a["settings.size"].split(" · ")[0], "85% – 120%", `${DEFAULTS.scale * 100}%`],
    [a["agent.title"], "Claude Code, Codex", DEFAULTS.agent ?? s["docs.firstInstalled"]],
  ];
  return `<div class="table"><table><thead><tr><th>${s["docs.setting"]}</th><th>${s["docs.options"]}</th><th>${s["docs.default"]}</th></tr></thead><tbody>${rows
    .map((r) => `<tr>${r.map((c) => `<td>${c}</td>`).join("")}</tr>`)
    .join("")}</tbody></table></div>`;
}

function genThemes(lang) {
  const a = APP[lang];
  return `<div class="themes">${THEMES.map(
    (th) => `<button class="theme-card card" data-theme="${th.id}" style="--tb:${th.colors.bg};--ts:${th.colors.surface};--tt:${th.colors.text};--ta:${th.colors.accent};--tm:${th.colors.muted}">
      <span class="swatch"><i></i><i></i><i></i></span><b>${th.name}</b><small>${th.dark ? a["settings.dark"] : a["settings.light"]}</small></button>`,
  ).join("")}</div>`;
}

function genShortcuts(lang) {
  const s = strings[lang];
  const keys = shortcutKeys();
  const missing = keys.filter((k) => !s[`shortcut.${k}`]);
  if (missing.length) throw new Error(`undocumented shortcuts (${lang}): ${missing.join(" ")} — add "shortcut.<key>" to site/i18n.json`);
  return `<div class="table"><table><thead><tr><th>${s["docs.shortcut"]}</th><th>${s["docs.action"]}</th></tr></thead><tbody>${keys
    .map((k) => `<tr><td><kbd>Ctrl</kbd> + <kbd>${esc(k.toUpperCase())}</kbd></td><td>${s[`shortcut.${k}`]}</td></tr>`)
    .join("")}</tbody></table></div>`;
}

function genThemeJson() {
  const { id, name, dark, colors } = THEMES[0];
  return `<pre data-lang="json"><code>${esc(JSON.stringify({ id: "my-theme", name: "My theme", dark, colors }, null, 2))}</code></pre>`;
}

// ---------- pages ----------

/** Theme variables for the site, in the app's names (--surface-2, --accent-text…). */
function themeCss() {
  const vars = (c) =>
    Object.entries(c).map(([k, v]) => `--${k.replace(/[A-Z]/g, (x) => "-" + x.toLowerCase()).replace("surface2", "surface-2")}:${v}`).join(";");
  const light = THEMES.find((t) => t.id === "paper");
  const dark = THEMES.find((t) => t.id === "paper-night");
  return [
    `:root{${vars(light.colors)}}`,
    `@media (prefers-color-scheme: dark){:root:not([data-theme]){${vars(dark.colors)}}}`,
    ...THEMES.map((t) => `:root[data-theme="${t.id}"]{${vars(t.colors)};color-scheme:${t.dark ? "dark" : "light"}}`),
  ].join("\n");
}

const fill = (tpl, vars) => tpl.replace(/\{\{([\w.]+)\}\}/g, (_, k) => {
  if (!(k in vars)) throw new Error(`missing string: ${k}`);
  return vars[k];
});

function build() {
  // Empty dist/ instead of deleting it: on Windows a shell sitting in it locks the folder.
  fs.mkdirSync(out, { recursive: true });
  for (const f of fs.readdirSync(out)) fs.rmSync(path.join(out, f), { recursive: true, force: true });
  const layout = read("site/src/layout.html");
  const pages = { landing: read("site/src/landing.html"), docs: read("site/src/docs.html") };

  for (const lang of LANGS) {
    const dir = lang === "en" ? "" : `${lang}/`;
    for (const page of ["landing", "docs"]) {
      const depth = (dir ? 1 : 0) + (page === "docs" ? 1 : 0);
      const base = "../".repeat(depth) || "./";
      const vars = {
        ...strings[lang],
        lang,
        base,
        home: `${base}${dir}`,
        docsUrl: `${base}${dir}docs/`,
        otherLang: `${base}${lang === "en" ? "pt/" : ""}${page === "docs" ? "docs/" : ""}`,
        version: JSON.parse(read("app/package.json")).version,
        url: SITE_URL + pagePath(lang, page),
        urlEn: SITE_URL + pagePath("en", page),
        urlPt: SITE_URL + pagePath("pt", page),
        siteUrl: SITE_URL,
        ogLocale: lang === "en" ? "en_US" : "pt_BR",
        description: strings[lang][page === "docs" ? "meta.docs" : "hero.sub"],
        navDocs: page === "docs" ? ' aria-current="page"' : "",
      };
      let body;
      if (page === "docs") {
        const { html, toc } = markdown(read(`site/content/${lang}/docs.md`));
        const gen = { cli: genCli(), settings: genSettings(lang), themes: genThemes(lang), shortcuts: genShortcuts(lang), themeJson: genThemeJson() };
        vars.content = html.replace(/\{\{(\w+)\}\}/g, (_, k) => gen[k] ?? `{{${k}}}`);
        vars.toc = toc.map((x) => `<a class="l${x.level}" href="#${x.id}">${x.text}</a>`).join("");
        body = fill(pages.docs, vars);
      } else {
        vars.themes = genThemes(lang);
        body = fill(pages.landing, vars);
      }
      vars.body = body;
      vars.title = page === "docs" ? `${strings[lang]["nav.docs"]} · dresume` : `dresume · ${strings[lang]["hero.short"]}`;
      vars.page = page;
      const file = path.join(out, dir, page === "docs" ? "docs" : "", "index.html");
      fs.mkdirSync(path.dirname(file), { recursive: true });
      fs.writeFileSync(file, fill(layout, vars));
    }
  }

  fs.writeFileSync(path.join(out, "theme.css"), themeCss() + "\n");
  for (const f of ["style.css", "site.js"]) fs.copyFileSync(path.join(site, "src", f), path.join(out, f));
  fs.cpSync(path.join(root, "docs/images"), path.join(out, "images"), { recursive: true });
  fs.copyFileSync(path.join(root, "assets/icon.svg"), path.join(out, "icon.svg"));
  fs.writeFileSync(path.join(out, ".nojekyll"), "");
  const today = new Date().toISOString().slice(0, 10);
  const urls = LANGS.flatMap((l) => ["landing", "docs"].map((p) => [l, p]));
  fs.writeFileSync(
    path.join(out, "sitemap.xml"),
    `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
${urls
      .map(([l, p]) => `<url><loc>${SITE_URL}${pagePath(l, p)}</loc><lastmod>${today}</lastmod>${LANGS.map((o) => `<xhtml:link rel="alternate" hreflang="${o}" href="${SITE_URL}${pagePath(o, p)}"/>`).join("")}</url>`)
      .join("\n")}
</urlset>
`,
  );
  fs.writeFileSync(path.join(out, "robots.txt"), `User-agent: *
Allow: /
Sitemap: ${SITE_URL}sitemap.xml
`);
  console.log(`site built: ${path.relative(root, out)} (${CLI.length} commands, ${THEMES.length} themes)`);
}

build();
