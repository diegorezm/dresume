# Changelog

All notable changes to dresume. The format follows [Keep a Changelog](https://keepachangelog.com/)
and the project uses [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-11-08

First release.

### Core
- Workspace model: a plain folder with `profile.yaml`, `templates/<id>/` and `resumes/<id>.yaml`.
- Templates are Typst layout + a JSON Schema; resumes are YAML content validated against it.
- Embedded Typst 0.15 compiler: PDF and per-page SVG output, diagnostics with file/line/column.
- Fonts from the system, Typst's embedded set and `<workspace>/fonts/`; Typst packages supported.
- Normalized profile for templates: plain URLs become links, scheme-less domains are treated as links,
  and each template can hide profile fields.
- "Basic" starter template written by `init` and offered when creating templates.

### CLI (`dresume`)
- Commands to list, show, validate, build, create and preview resumes and templates, with `--json` output.
- `profile-fields` to show or set which profile fields a template hides.
- `dresume mcp`: MCP stdio server (list, schema, show, validate, build, new_template, preview_template)
  for AI agents.

### Desktop app
- Home with template cards, resume list, create template/resume dialogs.
- Schema-driven resume form (sections, cards, bullets, tags, drag & drop reordering) with live preview
  and PDF export.
- Template code editor with tabs per file, Typst syntax highlighting and preview against sample data.
- Profile editor with a "show in templates" visibility table.
- Resizable editor/preview panes, editor/split/preview modes, collapsible sidebar (Ctrl+B), Ctrl+S,
  Ctrl+\; layouts adapt to narrow panes.
- Live reload of edits made outside the app.
- Settings: language (English, Portuguese), themes, accent, font, corners, scale; custom title bar and
  theme-aware scrollbars.

### Local AI agents
- Agent panel (Ctrl+J) for resumes and templates driving agents already installed on your machine:
  Claude Code, Codex, Gemini CLI, OpenCode and any ACP agent. No API keys.
- Model and reasoning effort picker, permission modes with in-app approvals.
- Review of every change the agent made, with keep or undo.
- Agents get the `dresume mcp` tools automatically.

### Installer
- Windows installer (NSIS and MSI) that also installs the `dresume` CLI and adds it to the user PATH.
- Release workflow building Windows, Linux and macOS bundles on new `v*` tags.

### Site
- Landing page and docs (English/Portuguese) generated from app sources, deployed to GitHub Pages.
