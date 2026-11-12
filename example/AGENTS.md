# dresume workspace — guide for AI agents

This folder is a dresume workspace: resumes written as YAML content, rendered by Typst templates.
Everything is plain files; edit them directly or use the `dresume` CLI.

## Layout

```
dresume.yaml            workspace marker (name, version)
profile.yaml            personal info shared by every resume (name, email, links…)
templates/<id>/
  template.yaml         display name, description, `profile.hide` (profile keys this template leaves out)
  template.typ          Typst layout
  schema.json           JSON Schema the resume content must satisfy
  sample.yaml           sample content used to preview the template
  *.typ                 helpers imported by template.typ
resumes/<id>.yaml       resume content; the `template: <id>` key links it to a template
fonts/                  optional extra fonts (.ttf/.otf)
memory.md               durable facts about the user, carried between chats (see Memory)
```

## Rules

- **Content goes in `resumes/*.yaml`, layout goes in `templates/`.** Don't put layout in a resume or content in a template.
- Every resume needs `template: <id>`, and its content must match `templates/<id>/schema.json`.
- Editing a template changes every resume linked to it. Run `dresume list` to see which ones are affected.
- Personal info shared across resumes belongs in `profile.yaml`, not in individual resumes.
- Ids (resume file names, template folder names) use only letters, digits, `-` and `_`.

## Memory

`memory.md` holds short facts about the user that should survive between chats, under three
sections: `## About the user`, `## Preferences`, `## Done`. One fact per `- fact (YYYY-MM-DD)` line.

- Save a fact when you learn something durable (background, a preference, something they did):
  `dresume remember <about|preferences|done> "<fact>"` or the MCP tool `remember`. One short fact per call.
- Contact info goes in `profile.yaml`, not in memory.
- The file stays under 4096 bytes: saving dedupes it and drops the oldest `Done` items. Lines with 📌 are never dropped.

## Template contract

Templates get their data through two inputs holding workspace-rooted paths:

```typst
#let profile = yaml(sys.inputs.profile)
#let resume  = yaml(sys.inputs.resume)
```

The profile templates receive is normalized: a link may be written in profile.yaml as a plain URL
(`website: https://you.dev`) or as `{label, url}`, and templates always get `{label, url}`. Fields listed in
the template's `profile.hide` are removed. Templates should render contact fields generically (iterate the
profile and skip what they show elsewhere) so new or hidden fields never break them.

When you add a field to a template, also add it to `schema.json` and `sample.yaml`.
These schema keywords drive the app's form editor: `title` (label), `format: "textarea"` (multi-line text), `enum` (select),
`format: "tags"` on a string array (chips instead of bullets), `items.title` (singular name, e.g. "Add position").

## CLI

Add `--json` to any command to get machine-readable output.

```
dresume list                                # templates and their resumes
dresume schema <template>                   # print a template's JSON Schema
dresume validate [ids…]                     # check resumes against their schema
dresume show <id>                           # print a resume as JSON
dresume build [ids…] [-o output]            # compile to PDF (all when no id is given)
dresume build <id> --format svg             # one SVG per page, to look at the result
dresume preview-template <template>         # compile a template with its sample.yaml
dresume new <id> --template <template>      # new resume from the template's sample
dresume duplicate <id> --from <resume>      # new resume with a copy of another one
dresume new-template <id> --name <name> [--from <template>|@starter]
dresume rename <id> <new-id>                # rename a resume file
dresume delete <id>                         # delete a resume
dresume edit-template <id> [--name <n>] [--description <d>] [--id <new-id>]  # new id relinks its resumes
dresume delete-template <id>                # deletes the template AND every resume linked to it
dresume remember <section> "<fact>"         # save a fact to memory.md (about, preferences, done)
dresume memory [--compact]                  # print memory.md and its size; --compact cleans it up
dresume profile-fields [<template> --hide phone,github]   # which profile fields each template shows
dresume mcp                                 # MCP server over stdio with the same tools (list, schema,
                                            # show, validate, build, new_template, preview_template, remember)
```

To add the tools to your agent: `claude mcp add dresume -- dresume -w <workspace> mcp` (or the equivalent
in your agent's MCP config). The dresume app does this for you when the CLI is on PATH.

Typical loop: edit YAML → `dresume validate --json` → `dresume build <id> --json` → fix the reported diagnostics.
