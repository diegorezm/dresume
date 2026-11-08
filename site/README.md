# dresume website

Landing page + docs (English and Portuguese), deployed to GitHub Pages by `.github/workflows/site.yml`.

```bash
node site/build.mjs        # → site/dist/  (no dependencies)
npx serve site/dist        # or any static server
```

- `src/` — page templates (`layout`, `landing`, `docs`), `style.css`, `site.js`
- `i18n.json` — landing/chrome strings per language
- `content/<lang>/docs.md` — docs text; `{{cli}}`, `{{settings}}`, `{{themes}}`, `{{themeJson}}`, `{{shortcuts}}` are generated

Generated from the sources: CLI reference (clap doc comments in `crates/cli/src/main.rs`), settings and their
labels/defaults (`theme.svelte.ts` + the app's `i18n.svelte.ts`), the site's colors and theme cards
(`BUILTIN_THEMES`), and keyboard shortcuts (`isShortcut` calls). The build fails if the app gains a shortcut
without a `shortcut.<key>` entry in `i18n.json`.
