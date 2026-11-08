// Basic: the starter template of every new workspace.
// Data comes from dresume: sys.inputs.resume and sys.inputs.profile.
#let profile = yaml(sys.inputs.profile)
#let resume  = yaml(sys.inputs.resume)

#let accent = rgb("#d9673f")
#let muted  = rgb("#6b6560")

#set page(paper: "a4", margin: (x: 1.6cm, y: 1.4cm))
// Libertinus Serif ships with Typst, so this renders the same on every machine.
#set text(font: "Libertinus Serif", size: 10.5pt)
#set par(justify: true, leading: 0.62em)
#show link: set text(fill: accent)

#show heading.where(level: 1): it => block(above: 1.1em, below: 0.6em)[
  #text(size: 10pt, weight: "bold", fill: accent, tracking: 0.08em, upper(it.body))
  #v(-0.5em)
  #line(length: 100%, stroke: 0.5pt + muted.lighten(60%))
]

#let entry(left, right) = grid(
  columns: (1fr, auto),
  column-gutter: 1em,
  text(weight: "bold", left), text(fill: muted, right),
)

// ── Header ────────────────────────────────────────────────────────────────────
#text(size: 24pt, weight: "bold", profile.name)
#v(-0.7em)
#text(size: 12pt, fill: muted, resume.target_role)
#v(-0.3em)
#{
  set text(size: 9pt)
  let parts = ()
  if "email" in profile { parts.push(link("mailto:" + profile.email, profile.email)) }
  if "phone" in profile { parts.push(profile.phone) }
  if "location" in profile { parts.push(profile.location) }
  for key in ("website", "github", "linkedin") {
    if key in profile { parts.push(link(profile.at(key).url, profile.at(key).label)) }
  }
  parts.join(text(fill: muted, "  ·  "))
}

= Summary
#resume.summary

= Skills
#resume.skills.join(text(fill: muted, "  ·  "))

= Experience
#for exp in resume.experience [
  #entry[#exp.role — #exp.company][#exp.period]
  #list(..exp.items, spacing: 0.5em)
  #v(0.3em)
]

#if "projects" in resume [
  = Projects
  #for p in resume.projects [
    #entry(if "link" in p { link(p.link, p.name) } else { p.name }, [])
    #list(..p.items, spacing: 0.5em)
    #v(0.3em)
  ]
]

#if "education" in resume [
  = Education
  #for e in resume.education [
    #entry[#e.degree — #e.institution][#e.period]
  ]
]
