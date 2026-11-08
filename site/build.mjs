// Builds the dresume website (landing + docs, en/pt) into site/dist/: `npm ci && node build.mjs` in site/.
// Reference sections are generated from the sources so the docs follow the code: the CLI from site/cli.json
// (exported from clap by a test in crates/cli), settings/themes/labels from the app. The build fails when a
// keyboard shortcut in the app is undocumented.
import fs from "node:fs";
import { Marked } from "marked";
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

const CLI = JSON.parse(read("site/cli.json"));

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

// ---------- markdown ----------

const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
const slug = (s) => s.toLowerCase().replace(/<[^>]+>/g, "").replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "");
const inline = (s) => new Marked().parseInline(s ?? "");

/** Markdown to HTML plus a table of contents. `## Title {#id}` sets the anchor, so both languages share it. */
function markdown(src) {
  const toc = [];
  const md = new Marked({
    renderer: {
      heading({ tokens, depth, text }) {
        const custom = text.match(/ \{#([\w-]+)\}$/)?.[1];
        const html = this.parser.parseInline(tokens).replace(/ \{#[\w-]+\}$/, "");
        const id = custom ?? slug(html);
        if (depth === 2 || depth === 3) toc.push({ level: depth, id, text: html });
        return `<h${depth} id="${id}"><a class="anchor" href="#${id}">${html}</a></h${depth}>\n`;
      },
      code({ text, lang }) {
        return `<pre${lang ? ` data-lang="${lang}"` : ""}><code>${esc(text)}</code></pre>\n`;
      },
      table(token) {
        return `<div class="table">${this.constructor.prototype.table.call(this, token)}</div>`;
      },
    },
  });
  return { html: md.parse(src), toc };
}

// ---------- generated sections ----------

function usage(a) {
  const name = a.positional ? (a.multiple ? `${a.name}…` : `<${a.name}>`)
    : [a.short && `-${a.short}`, `--${a.long}`].filter(Boolean).join(", ") + (a.flag ? "" : ` <${a.name}>`);
  return a.required ? name : `[${name}]`;
}

function genCli() {
  const flag = (a) => `<li><code>${esc(usage(a).replace(/^\[|\]$/g, ""))}</code> ${inline(a.help)}${a.default ? ` <span class="muted">(default: <code>${esc(a.default)}</code>)</span>` : ""}</li>`;
  const global = `<div class="cmd card global"><p><b>${esc(CLI.about)}.</b></p><ul>${CLI.global.map(flag).join("")}</ul></div>`;
  return global + CLI.commands.map((c) => {
    const opts = c.args.filter((a) => a.help || a.default);
    return `<div class="cmd card" id="cli-${c.name}"><pre><code>${esc(["dresume", c.name, ...c.args.map(usage)].join(" "))}</code></pre><p>${inline(c.about)}.</p>${
      opts.length ? `<ul>${opts.map(flag).join("")}</ul>` : ""
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
  const olive = THEMES.find((t) => t.id === "sage").colors.accent;
  return [
    `:root{${vars(light.colors)};--olive:${olive}}`,
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
        vars.content = html.replace(/(?:<p>)?\{\{(\w+)\}\}(?:<\/p>)?/g, (_, k) => gen[k] ?? `{{${k}}}`);
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
  console.log(`site built: ${path.relative(root, out)} (${CLI.commands.length} commands, ${THEMES.length} themes)`);
}

build();
