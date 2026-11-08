#import "helpers.typ": contact-row
#import "theme.typ": setup-page, muted-color

// Data comes from dresume: sys.inputs.resume and sys.inputs.profile.
#let profile = yaml(sys.inputs.profile)
#let resume  = yaml(sys.inputs.resume)

#show: setup-page

// ── Header ────────────────────────────────────────────────────────────────────
#align(center)[
  #text(size: 22pt, weight: "bold")[#profile.name]
  #v(0.1em)
  #text(size: 11pt, fill: muted-color)[#resume.target_role]
  #v(0.4em)
  #text(size: 8.5pt)[#contact-row(profile)]
]

#v(0.6em)

// ── Summary ───────────────────────────────────────────────────────────────────
= Summary
#resume.summary

// ── Skills ────────────────────────────────────────────────────────────────────
= Skills
#resume.skills.join(" • ")

// ── Experience ────────────────────────────────────────────────────────────────
= Experience
#for exp in resume.experience [
  #grid(
    columns: (1fr, auto),
    align(left)[== #exp.role — #exp.company],
    align(right)[#text(fill: muted-color)[#exp.period]],
  )
  #v(0.15em)
  #for item in exp.items [
    - #item
  ]
  #v(0.3em)
]

// ── Projects ──────────────────────────────────────────────────────────────────
#if "projects" in resume [
  = Projects
  #for project in resume.projects [
    #grid(
      columns: (1fr, auto),
      align(left)[
        == #if "link" in project [
          #link(project.link)[#project.name]
        ] else [
          #project.name
        ]
      ],
      align(right)[],
    )
    #v(0.15em)
    #for item in project.items [
      - #item
    ]
    #v(0.3em)
  ]
]

// ── Education ─────────────────────────────────────────────────────────────────
#if "education" in resume [
  = Education
  #for edu in resume.education [
    #grid(
      columns: (1fr, auto),
      align(left)[== #edu.degree — #edu.institution],
      align(right)[#text(fill: muted-color)[#edu.period]],
    )
    #v(0.3em)
  ]
]
