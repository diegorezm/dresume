# Documentation

dresume keeps your resume **content** in YAML and the **layout** in Typst templates. Change a template and
every resume linked to it is restyled. This page covers the desktop app, the workspace format, templates and the CLI.

## Get started {#get-started}

> Prebuilt installers are coming soon. For now, build from source.

You need [Rust](https://rustup.rs/), [Node.js](https://nodejs.org/) and [pnpm](https://pnpm.io/).
Typst does **not** need to be installed: it's built in.

```bash
git clone https://github.com/diegorezm/dresume
cd dresume/app
pnpm install
pnpm tauri dev
```

Click **New workspace**, pick a folder, and start editing `my-resume`.

### Command line only

```bash
cargo install --path crates/cli
dresume init my-resumes
dresume -w my-resumes build          # → my-resumes/output/*.pdf
```

## Concepts {#concepts}

A **workspace** is a plain folder. There is no database and no hidden state: everything is a file you can
read, edit, diff and commit.

```
my-resumes/
├── dresume.yaml        workspace marker (name, version)
├── profile.yaml        name, email, links — shared by every resume
├── templates/<id>/
│   ├── template.yaml   display name, description, hidden profile fields
│   ├── template.typ    the Typst layout
│   ├── schema.json     which content the template accepts
│   └── sample.yaml     sample content used to preview the template
├── resumes/<id>.yaml   content only, with `template: <id>`
└── fonts/              optional extra fonts (.ttf/.otf)
```

- A **template** is layout plus a JSON Schema describing the content it accepts.
- A **resume** is content only and links to a template with `template: <id>`.
- The **profile** holds personal info once. Each template can hide profile fields it doesn't want.
- Ids (file and folder names) use only letters, digits, `-` and `_`.

## Using the app {#app}

### Overview

The home screen lists your templates as cards, each with the resumes that use it. From a card you can create
a resume with that template or open the template's layout.

### Editing a resume

Resumes are edited with a form generated from the template's schema: basic fields on top, then one card per
section. Drag items to reorder them, add bullets and tags, and add optional sections when you need them. The
preview updates while you type, before you save. Export to PDF from the top bar.

### Editing a template

The template editor has a tab per file (`template.typ`, helpers, `schema.json`, `sample.yaml`) with Typst
highlighting. The preview renders the template with its own `sample.yaml`, and errors point to the file and line.

### Profile

The profile editor edits `profile.yaml`. Links can be plain URLs or `{label, url}`. The **show in templates**
table chooses which fields each template displays.

### AI agent panel

Open it with <kbd>Ctrl</kbd>+<kbd>J</kbd> in the resume editor. It drives the AI agent you already have
installed (Claude Code or Codex): ask for a change and the agent edits the workspace files directly while
the preview follows. dresume never calls a model API itself.

## Settings {#settings}

Settings are saved per user, in the app's config folder, not in the workspace.

{{settings}}

### Themes

{{themes}}

### Custom themes

Use **Save as a new theme** to keep the current theme with your accent color, or drop a JSON file in the
themes folder (**Open themes folder** in Settings). It has the same shape as the built-in themes:

{{themeJson}}

## Keyboard shortcuts {#shortcuts}

On macOS use <kbd>Cmd</kbd> instead of <kbd>Ctrl</kbd>.

{{shortcuts}}

## Writing templates {#templates}

Templates get their data through two inputs holding workspace-rooted paths:

```typst
#let profile = yaml(sys.inputs.profile)
#let resume  = yaml(sys.inputs.resume)
```

The profile a template receives is normalized: links are always `{label, url}` and fields hidden by the
template are removed. Render contact fields generically (iterate the profile) so new or hidden fields never
break the layout. When you add a field, also add it to `schema.json` and `sample.yaml`.

Start from **Basic** (`--from @starter`) or copy an existing template. For fonts that work everywhere, use the
ones built into Typst: Libertinus Serif, New Computer Modern, DejaVu Sans Mono. Others go in `fonts/`.

### Schema and the form

These schema keywords drive the app's form:

| Keyword | Effect |
|---|---|
| `title` | Field label |
| `format: "textarea"` | Multi-line text |
| `enum` | Select |
| `format: "uri"` | URL input |
| array of strings | Bullets |
| `format: "tags"` on a string array | Chips instead of bullets |
| array of objects | Collapsible cards, titled by the first two text fields |
| `period` / `date` / `year` key | Shown as the card's meta |
| `items.title` | Singular noun ("Add position") |

Non-array top-level fields are grouped in the **Basics** card; optional sections appear under **Add section**.

## CLI reference {#cli}

Every command accepts `-w <dir>` (workspace; defaults to the nearest parent with `dresume.yaml`) and `--json`
for machine-readable output. Commands exit non-zero on failure.

{{cli}}

## For AI agents {#agents}

Every workspace includes an `AGENTS.md` that teaches agents the format. A typical loop:

1. Edit the YAML.
2. `dresume validate --json` to check it against the schema.
3. `dresume build <id> --format svg --json` to render pages the agent can look at.
4. Fix the reported diagnostics.

Try prompts like *"tailor my backend resume for this job post"* or *"make a two-column template with a dark sidebar"*.
