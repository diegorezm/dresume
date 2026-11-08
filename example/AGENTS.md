# dresume workspace — guide for AI agents

This folder is a dresume workspace: resumes written as YAML content, rendered by Typst templates.
Everything is plain files; edit them directly or use the `dresume` CLI.

## Layout

```
dresume.yaml            workspace marker (name, version)
profile.yaml            personal info shared by every resume (name, email, links…)
templates/<id>/
  template.yaml         display name + description
  template.typ          Typst layout
  schema.json           JSON Schema the resume content must satisfy
  sample.yaml           sample content used to preview the template
  *.typ                 helpers imported by template.typ
resumes/<id>.yaml       resume content; the `template: <id>` key links it to a template
fonts/                  optional extra fonts (.ttf/.otf)
```

## Rules

- **Content goes in `resumes/*.yaml`, layout goes in `templates/`.** Don't put layout in a resume or content in a template.
- Every resume needs `template: <id>`, and its content must match `templates/<id>/schema.json`.
- Editing a template changes every resume linked to it. Run `dresume list` to see which ones are affected.
- Personal info shared across resumes belongs in `profile.yaml`, not in individual resumes.
- Ids (resume file names, template folder names) use only letters, digits, `-` and `_`.

## Template contract

Templates get their data through two inputs holding workspace-rooted paths:

```typst
#let profile = yaml(sys.inputs.profile)
#let resume  = yaml(sys.inputs.resume)
```

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
```

Typical loop: edit YAML → `dresume validate --json` → `dresume build <id> --json` → fix the reported diagnostics.
