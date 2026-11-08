#import "@preview/fontawesome:0.6.0": *

// Renders a single contact item with an icon and optional link.
#let contact-item(icon, label, url: none, brand: false) = {
  let i = if brand {
    fa-icon(icon, font: "Font Awesome 7 Brands")
  } else {
    fa-icon(icon)
  }
  if url != none {
    link(url)[ #i #h(0.3em) #label ]
  } else {
    [ #i #h(0.3em) #label ]
  }
}

// Icon per known profile field; other fields are shown without an icon.
#let icons = (
  email: ("envelope", false),
  phone: ("phone", false),
  location: ("location-dot", false),
  linkedin: ("linkedin", true),
  github: ("github", true),
  website: ("globe", false),
)

// Full contact row from the profile, in file order. Hidden or missing fields are skipped.
#let contact-row(profile) = {
  let items = ()
  for (key, value) in profile.pairs() {
    if key in ("name", "title", "photo") or type(value) == array { continue }
    let (icon, brand) = icons.at(key, default: (none, false))
    let (label, url) = if type(value) == dictionary and "url" in value {
      (value.label, value.url)
    } else if key == "email" {
      (value, "mailto:" + value)
    } else {
      (str(value), none)
    }
    items.push(if icon == none {
      if url == none { label } else { link(url, label) }
    } else {
      contact-item(icon, label, url: url, brand: brand)
    })
  }
  items.join(h(1em))
}
