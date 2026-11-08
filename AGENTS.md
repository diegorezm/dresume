# AGENTS.md — contributing to dresume

Technical guide for AI agents (and humans) changing this repository.
For the guide that ships *inside user workspaces* (how to edit resumes/templates), see
[`crates/core/assets/AGENTS.md`](crates/core/assets/AGENTS.md). Don't confuse the two.

## What this is

dresume is a resume manager built on [Typst](https://typst.app/):

- A **workspace** is a plain folder: `profile.yaml`, `templates/<id>/`, `resumes/<id>.yaml`.
- A **template** is layout (Typst) + a JSON Schema describing the content it accepts.
- A **resume** is content only (YAML) and links to a template with `template: <id>`.
- Changing a template restyles every resume linked to it.

Three front doors over the same library: a Rust **core**, a **CLI** (`dresume`, JSON output and an MCP
server for agents), and a **desktop app** (Tauri 2 + SvelteKit/Svelte 5). The app also drives the AI agents
already installed on the user's machine (Claude Code, Codex, Gemini CLI, OpenCode): it never calls a model
API itself.

## Repository map

```
Cargo.toml                    Cargo workspace (core, cli, app/src-tauri) + shared deps + release profile
crates/
  core/                       dresume-core: ALL domain logic lives here
    src/lib.rs                public API re-exports + workspace format doc
    src/workspace.rs          Workspace: discover/init/open, templates, resumes, schema validation, starter
    src/world.rs              Typst `World` impl: fonts (system+embedded+<ws>/fonts), files, packages, overlay
    src/compile.rs            compile() / compile_template_preview() -> PDF and/or per-page SVG + Diagnostics
    src/highlight.rs          Typst syntax highlighting (typst-syntax) with UTF-16 offsets for the editor
    src/profile.rs            profile handed to templates: link normalization + per-template `profile.hide`
    assets/AGENTS.md          guide copied into every new workspace (user-facing, agent-oriented)
    assets/starter/           "Basic" template embedded with include_str! and written by `init`
  agent/                      dresume-agent: spawns local agent CLIs, one AgentEvent stream for all
    src/lib.rs                AgentKind, detect(), start()/start_acp(), Session, AgentEvent, PermissionMode
    src/claude.rs             `claude -p` stream-json; approvals via control_request can_use_tool
    src/codex.rs              `codex app-server` JSON-RPC; approvals as server requests
    src/acp.rs                Agent Client Protocol (gemini --acp, opencode acp, any ACP agent)
    src/rpc.rs                tiny JSON-RPC helper shared by codex + acp
    src/process.rs            PATH lookup, `.cmd` shims, no console windows, process-tree kill
    src/snapshot.rs           workspace snapshot → diffs of what the agent changed + undo
    examples/chat.rs          manual run against a real agent (`--example chat -- claude <dir> "<prompt>"`)
  cli/src/main.rs             `dresume` binary (clap). Every command supports --json
  cli/src/mcp.rs              `dresume mcp`: MCP stdio server (list, schema, show, validate, build, new_template, preview_template)
app/                          desktop app
  src-tauri/src/lib.rs        Tauri commands: thin wrappers over dresume-core + settings/themes I/O
  src-tauri/src/agent.rs      agent commands (start/send/respond/cancel/changes/keep/undo) + chat history
  src-tauri/src/watch.rs      workspace watcher → `workspace-changed` event (live reload of outside edits)
  src-tauri/tauri.conf.json   window config (custom title bar: decorations=false, dragDropEnabled=false)
  src-tauri/capabilities/     Tauri permissions; add window/plugin permissions here
  src/routes/+layout.svelte   loads app.css, settings/theme, title bar; dev IPC mock outside Tauri
  src/routes/+page.svelte     shell: welcome screen, sidebar, home (template cards), view routing, dialogs
  src/app.css                 global styles + shared classes (.card, .topbar, .chip, .page-head, buttons)
  src/lib/api.ts              typed `invoke` wrappers for every Tauri command + shared types + slug()
  src/lib/theme.svelte.ts     themes (built-ins), settings model, applies CSS variables
  src/lib/i18n.svelte.ts      UI translations (en = source of truth, pt) and t()
  src/lib/ResumeEditor.svelte content editor + live preview + PDF export + agent panel (Ctrl+J)
  src/lib/AgentPanel.svelte   chat with the local agent (resume or template): tool calls, approvals, diff + keep/undo
  src/lib/AgentPane.svelte    resizable side pane hosting AgentPanel (resume and template editors, Ctrl+J)
  src/lib/TemplateEditor.svelte template code editor (tabs per file) + preview against sample.yaml
  src/lib/ProfileEditor.svelte profile.yaml editor + "show in templates" visibility table
  src/lib/CodeEditor.svelte   CodeMirror 6 wrapper, themed through CSS variables
  src/lib/typstHighlight.ts   CodeMirror extension calling the highlight_typst command
  src/lib/Preview.svelte      SVG pages, zoom, diagnostics list
  src/lib/SplitPane.svelte    resizable two-pane layout; ratio in localStorage, falls back to one pane when narrow
  src/lib/ViewSwitch.svelte   editor / split / preview switch for a SplitPane
  src/lib/layout.svelte.ts    layout prefs (sidebar visible, pane mode per editor) + isShortcut()
  src/lib/form/               schema-driven content form (ResumeForm > Field > ItemList/BulletList/TagInput)
  src/lib/Modal.svelte        shared dialog shell + .m-* classes for dialog bodies
  src/lib/New*Dialog.svelte   create template / create resume dialogs
  src/lib/SettingsView.svelte language, theme, accent, font, corners, scale, AI agent + permission mode
  src/lib/TitleBar.svelte     custom window title bar
  src/lib/dev/mock.ts         fake backend for `pnpm dev` in a browser (data from fixture.json)
  scripts/dev-fixture.mjs     builds src/lib/dev/fixture.json from example/ using the CLI
example/                      sample workspace (classic + academic templates, 4 resumes)
docs/images/                  README images (rendered with the typst CLI)
```

## Architecture rules

1. **Domain logic goes in `dresume-core`.** The CLI and the Tauri app only parse input, call core, and
   format output. If you need the same behavior in two places, it belongs in core.
2. **The workspace is the API.** Everything must stay a readable/editable plain file. No databases, no hidden
   state in the workspace. Per-user app state (theme, language, last workspace) lives in the app config dir
   (`settings.json`, `themes/*.json`), never in the workspace.
3. **Templates are data-driven.** A template reads `sys.inputs.resume` and `sys.inputs.profile`
   (workspace-rooted paths). The app's form is generated from the template's `schema.json`. Don't
   special-case a template in app code.
4. **Templates never read profile.yaml directly.** Core writes a normalized copy (plain URLs become
   `{label, url}`, the template's `profile.hide` keys removed) into the overlay at `.dresume/profile.yaml`
   and passes that path. Templates render contact fields generically so added/hidden fields can't break them.
5. **Unsaved edits use the overlay.** Live preview passes in-memory file contents
   (`Overlay`: workspace-relative path → text) instead of writing to disk.
6. **Agent-friendly by default.** New CLI commands support `--json`, return non-zero on failure, and are
   documented in `crates/core/assets/AGENTS.md` (then copied to `example/AGENTS.md`). Tools agents
   should call often also belong in `dresume mcp`.
7. **Local agents only.** The app spawns agent CLIs the user installed and logged into; no API keys, no
   model calls from dresume. Each driver maps its protocol to `AgentEvent` so the UI never special-cases an
   agent. The agent works on the real files; the snapshot gives review and undo, the watcher the preview.

## Conventions

### Rust
- Edition 2024, workspace dependencies in the root `Cargo.toml` (`dep.workspace = true` in members).
- Errors: `anyhow::Result` in core and CLI. Tauri commands return `Result<T, String>` via the
  `err()` / `with_ws()` helpers in `app/src-tauri/src/lib.rs`.
- Ids (resume/template names) are validated by `check_id`: `[A-Za-z0-9_-]+`. Keep that rule everywhere.
- Diagnostics carry workspace-relative `file` + 1-based `line`/`column`; package files have `file: None`.
- Typst crates are pinned to the same version (`0.15.1`). Upgrade them together.

### Frontend (Svelte 5)
- Runes only (`$state`, `$derived`, `$effect`, `$props`). Shared reactive state lives in `*.svelte.ts`
  classes (`appearance`, `i18n`).
- Every backend call goes through `src/lib/api.ts`. Add the typed wrapper there, never call `invoke`
  directly from components (exceptions: settings/theme plumbing in `theme.svelte.ts`, highlighting).
- **No hard-coded colors.** Use the theme variables: `--bg --sidebar --surface --surface-2 --canvas --border
  --text --muted --accent --accent-soft --accent-text --danger --danger-bg --warn --shadow --radius
  --radius-sm --font --mono`. Derive shades with `color-mix()`.
- **No hard-coded UI text.** Add a key to `en` in `src/lib/i18n.svelte.ts`, translate it in `pt`
  (TypeScript fails if a key is missing), and use `t("key", { params })`. Plurals are functions.
  Don't name loop variables `t` in components: it shadows the translator (use `tpl`, `th`, `x`).
- Buttons: default pill; `.primary` for the main action, `.ghost` for icon-like actions, `.danger` for
  destructive ones. Reuse `.card`, `.topbar`, `.chip`, `.page-head`, `.bubble` from `app.css`.
- Layout must adapt: editors put the topbar outside `SplitPane` (it must survive "preview only"), forms
  use container queries (`.form-scroll` is an inline-size container) instead of viewport media queries.
  Shortcuts: Ctrl+S save, Ctrl+\ toggle preview, Ctrl+B toggle sidebar; use `isShortcut()`.
- Dialogs use `Modal.svelte` and its `.m-*` classes. Respect `prefers-reduced-motion` in animations.

### Template schema conventions (drive the form UI)
- `title` → field label; `format: "textarea"` → multi-line; `enum` → select; `format: "uri"` → URL input.
- Arrays of strings: bullets by default, `format: "tags"` → chips.
- Arrays of objects → collapsible cards; title = first two text fields, meta = a `period`/`date`/`year` key;
  `items.title` sets the singular noun ("Add position").
- Non-array top-level fields are grouped in the "Basics" card; optional sections show as "Add section".

## Common tasks

**Add a Tauri command**
1. Implement the logic in `dresume-core` (and a CLI subcommand if agents should have it).
2. Add a `#[tauri::command]` wrapper in `app/src-tauri/src/lib.rs` and register it in `generate_handler!`.
3. Add the typed wrapper to `app/src/lib/api.ts`.
4. If it needs new window/plugin permissions, add them to `app/src-tauri/capabilities/default.json`.
5. If the browser dev mock should support it, handle it in `src/lib/dev/mock.ts`.

**Add a built-in theme:** append to `BUILTIN_THEMES` in `src/lib/theme.svelte.ts` (all `colors` keys required).
User themes are JSON files with the same shape in `<app config>/themes/`.

**Add a language:** add a dictionary typed `Record<Key, Entry>` in `i18n.svelte.ts`, register it in
`LOCALES` and `dictionaries`, and update `systemLocale()`.

**Add an agent:** if it speaks ACP, add an `AgentKind` variant that calls `acp::run(program, args, …)`.
Otherwise write a driver module (see `codex.rs`): parse its stdout, emit `AgentEvent`s, map approvals to
`Permission` events and honor `PermissionMode`; pass `SessionOptions::mcp` in its config format. Add the
kind to `AgentKind` in `api.ts`, then check it live with `cargo run -p dresume-agent --example chat`.

**Change the starter template:** edit `crates/core/assets/starter/*`. It's written by `Workspace::init` and
offered as `--from @starter` / "Basic" when creating templates.

## Build, run, check

```bash
# Rust
cargo build --workspace
cargo test -p dresume-core -p dresume-agent
cargo run -p dresume-cli -- -w example list
cargo run -p dresume-cli -- -w example build            # PDFs into example/output/
cargo run -p dresume-cli -- -w example preview-template academic

# App
cd app
pnpm install
pnpm check                  # svelte-check + TypeScript; must be 0 errors, 0 warnings
pnpm tauri dev              # desktop app
node scripts/dev-fixture.mjs && pnpm dev   # UI in a browser with a mocked backend
```

Before finishing a change: `cargo build --workspace`, `cargo test -p dresume-core`, `pnpm check`, and
`dresume -w example validate` if you touched schemas or the example.

## Gotchas

- `serde_json` uses `preserve_order`: schema and resume key order drives the form and the YAML output. Don't
  drop the feature or round-trip through sorted maps.
- CodeMirror stores LF; `CodeEditor.svelte` compares normalized text so CRLF files don't look edited.
- README screenshots come from `pnpm dev` with the mock (`scripts/dev-fixture.mjs` uses a neutral name/path).
- `tauri.conf.json` and capability changes need a restart of `pnpm tauri dev` (no hot reload).
- `dragDropEnabled: false` is required for HTML5 drag & drop (item reordering) on Windows.
- The editor gets UTF-16 offsets from `highlight_typst`; Typst itself works in bytes. Keep the conversion.
- Typst pitfalls when writing templates: there is no `??` operator; a method chain broken across lines in
  markup mode becomes literal text (keep it on one line or wrap it in `{ }`).
- Fonts: system + Typst's embedded set + `<workspace>/fonts/`. Templates that should work everywhere use
  embedded fonts (Libertinus Serif, New Computer Modern, DejaVu Sans Mono).
- In Git Bash on Windows, set `MSYS_NO_PATHCONV=1` before passing `/path` inputs to the `typst` CLI.
- Agent CLIs installed by npm are `.cmd` shims on Windows: they run through `cmd /C`, which re-parses
  arguments. Pass long text (instructions, MCP config) through files or stdin, never as arguments.
- `canonicalize` adds `\\?\` on Windows; agents get the plain path (`plain_path`) or relative titles break.
- Claude's `--permission-prompt-tool stdio` is what makes approvals arrive on stdout; without it, edits in
  `default` mode are just denied.

## License

AGPL-3.0-or-later. Contributions are accepted under the same license.
