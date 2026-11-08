// Academic CV: serif, single column, LaTeX-style.
// Data comes from dresume: sys.inputs.resume and sys.inputs.profile.
#let profile = yaml(sys.inputs.profile)
#let resume  = yaml(sys.inputs.resume)

#let accent = rgb("#7a1f1f")   // burgundy
#let muted  = luma(90)

#set document(title: profile.name + " — CV", author: profile.name)
#set page(
  paper: "a4",
  margin: (x: 2cm, y: 1.8cm),
  footer: context {
    set text(8pt, fill: muted)
    profile.name
    h(1fr)
    counter(page).display("1 / 1", both: true)
  },
)
#set text(font: "New Computer Modern", size: 10.5pt, lang: resume.at("lang", default: "en"))
#set par(justify: true, leading: 0.6em)
#show link: set text(fill: accent)

#show heading.where(level: 1): it => {
  v(0.8em, weak: true)
  block(below: 0.55em, stroke: (bottom: 0.5pt + accent), inset: (bottom: 0.25em), width: 100%,
    text(size: 11pt, fill: accent, smallcaps(it.body)))
}

// An entry with a bold left title, right-aligned date and optional body.
#let entry(title, date: none, subtitle: none, body: none) = block(breakable: false, below: 0.6em)[
  #grid(
    columns: (1fr, auto),
    column-gutter: 1em,
    strong(title), text(fill: muted, if date == none [] else [#date]),
  )
  #if subtitle != none [#v(-0.4em) #emph(subtitle)]
  #if body != none [#v(-0.2em) #body]
]

#let bullets(items) = if items != none and items.len() > 0 {
  list(..items, indent: 0.4em, body-indent: 0.5em, spacing: 0.45em)
}

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

// Bolds the CV owner's name inside an author list.
#let authors(names) = names.map(a => if a == profile.name { strong(a) } else { a }).join(", ", last: " and ")

#let section(key) = resume.at(key, default: none) != none and resume.at(key).len() > 0

// ── Header ────────────────────────────────────────────────────────────────────
#align(center)[
  #text(size: 22pt, smallcaps(profile.name))
  #if "position" in resume [
    #v(-0.6em)
    #text(size: 11.5pt, fill: muted, resume.position)
  ]
  #v(-0.3em)
  #set text(size: 9.5pt)
  #contacts(profile).join([ #h(0.4em) · #h(0.4em) ])

]

#if "summary" in resume [
  #v(0.4em)
  #resume.summary
]

#if section("research_interests") [
  = Research Interests
  #resume.research_interests.join(" · ")
]

#if section("education") [
  = Education
  #for e in resume.education {
    let details = ()
    if "thesis" in e { details.push([_Thesis:_ #e.thesis]) }
    if "advisor" in e { details.push([_Advisor:_ #e.advisor]) }
    details += e.at("items", default: ())
    entry(e.degree, date: e.period, subtitle: e.institution, body: bullets(details))
  }
]

#if section("publications") [
  = Publications
  #enum(
    numbering: n => text(fill: muted, "[" + str(n) + "]"),
    spacing: 0.7em,
    ..resume.publications.map(p => [
      #authors(p.authors).
      #if "link" in p { link(p.link, ["#p.title."]) } else ["#p.title."]
      #emph(p.venue), #p.year.
      #if "note" in p [ #text(fill: muted, p.note)]
    ]),
  )
]

#if section("experience") [
  = Experience
  #for x in resume.experience {
    entry(x.role, date: x.period, subtitle: x.organization, body: bullets(x.at("items", default: ())))
  }
]

#if section("teaching") [
  = Teaching
  #for t in resume.teaching {
    entry(t.course, date: t.period, subtitle: t.role + ", " + t.institution)
  }
]

#if section("awards") [
  = Awards & Grants
  #for a in resume.awards {
    grid(
      columns: (1fr, auto),
      column-gutter: 1em,
      [*#a.title* #if "issuer" in a [— #a.issuer]],
      text(fill: muted, str(a.year)),
    )
    v(0.2em)
  }
]

#if section("talks") [
  = Talks
  #for t in resume.talks {
    grid(
      columns: (1fr, auto),
      column-gutter: 1em,
      [_"#t.title"_, #t.event],
      text(fill: muted, str(t.year)),
    )
    v(0.2em)
  }
]

#if section("skills") [
  = Skills
  #grid(
    columns: (auto, 1fr),
    column-gutter: 1em,
    row-gutter: 0.6em,
    ..resume.skills.map(s => (strong(s.category), s.items.join(", "))).flatten(),
  )
]

#if section("languages") [
  = Languages
  #resume.languages.map(l => [*#l.name* (#l.level)]).join(" · ")
]
