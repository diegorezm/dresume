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

// One contact entry from the profile. dresume turns plain URLs into {label, url}.
#let contact(value) = {
  if type(value) == dictionary and "url" in value { link(value.url, value.label) }
  else if type(value) == str and value.match(regex("^[^@\s]+@[^@\s]+\.[^@\s]+$")) != none { link("mailto:" + value, value) }
  else if type(value) in (str, int, float) { str(value) }
}

// Every profile field except the ones rendered elsewhere, in file order.
#let contacts(profile, skip: ("name", "title", "photo")) = (
  profile
    .pairs()
    .filter(((key, value)) => key not in skip and type(value) != array)
    .map(((key, value)) => contact(value))
    .filter(c => c != none)
)

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
  contacts(profile).join(text(fill: muted, "  ·  "))
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
