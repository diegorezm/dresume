// Global visual settings: margins, fonts, colors, and heading styles.
// Edit this file to change the look of all resumes at once.

#let accent-color = rgb("#2563EB")   // blue
#let muted-color  = rgb("#64748B")   // gray
#let link-color   = rgb("#2563EB")

#let setup-page(body) = {
  set page(
    paper: "a4",
    margin: (x: 1.4cm, y: 1.2cm),
  )
  set text(
  font: "Libertinus Sans",
    size: 10pt,
    lang: "pt",
  )
  set par(justify: true, leading: 0.65em)

  show link: it => text(fill: link-color)[#it]

show heading.where(level: 1): set block(above: 0.6em, below: 0.6em)
  show heading.where(level: 1): it => [
    #text(weight: "bold", size: 11.5pt)[#upper(it.body)]
    #line(length: 100%, stroke: 0.4pt + muted-color)
    #v(0.15em)
  ]

  show heading.where(level: 2): it => [
    #v(0.05em)
    #text(size: 10pt, weight: "semibold")[#it.body]
  ]

  body
}
