// A setlist on one page: the type size is the largest that fits (between the
// theme's largest and smallest size). The data comes from bp-print/src/data.rs.
// Themes: "stage" (for the floor: big titles, keys, transitions), "technical"
// (FOH: every column with times and cues), "elegant" (serif, centred).

#let data = json(bytes(sys.inputs.data))
#let logo = sys.inputs.at("logo", default: none)

#let tints = (
  red: rgb("#f8c9c4"), orange: rgb("#fbd9b0"), yellow: rgb("#f8eba0"),
  green: rgb("#c9ebc4"), blue: rgb("#c4dcf5"), purple: rgb("#dcccf0"),
)
#let tint(name) = if name != "" { tints.at(name, default: none) } else { none }
#let muted = luma(95)
#let mono(body) = text(font: "JetBrains Mono", body)
#let serif = "Source Serif 4"

#let fits = (
  stage: (max: 48pt, min: 13pt),
  technical: (max: 10pt, min: 6pt),
  elegant: (max: 24pt, min: 9pt),
)
#let theme = if data.theme in fits { data.theme } else { "technical" }
#let range = fits.at(theme)

#set document(title: data.title + if data.band != "" { " – " + data.band } else { "" })
#set page(
  paper: data.paper,
  flipped: data.landscape,
  margin: if theme == "stage" { (x: 1.2cm, top: 1.1cm, bottom: 1.3cm) } else { (x: 1.5cm, top: 1.4cm, bottom: 1.5cm) },
  footer: context {
    set text(font: if theme == "elegant" { serif } else { "Inter" }, size: 7pt, fill: muted)
    grid(
      columns: (1fr, auto),
      [#data.summary],
      [#data.printed#if counter(page).final().first() > 1 [ · Page #counter(page).display() / #counter(page).final().first()]],
    )
  },
)
#set text(font: if theme == "elegant" { serif } else { "Inter" }, lang: "en")
#set par(leading: 0.35em, spacing: 0.4em)

// The head: logo, band, setlist name, where and when.
#let head(s) = {
  let place = (data.venue, data.date, if data.start != "" { "Start " + data.start } else { "" }).filter(x => x != "")
  if theme == "elegant" {
    align(center, {
      if logo != none { image(logo, height: 3.2 * s, fit: "contain"); v(0.3 * s) }
      if data.band != "" { text(size: 1.5 * s, weight: "semibold", tracking: 0.08em, upper(data.band)); linebreak() }
      text(size: 1.05 * s, style: "italic", data.title)
      if place.len() > 0 { linebreak(); text(size: 0.7 * s, fill: muted, place.join("  ·  ")) }
      v(0.2 * s)
      line(length: 30%, stroke: 0.5pt + muted)
    })
  } else {
    let k = if theme == "stage" { 0.42 } else { 1.0 }
    grid(
      columns: if logo != none { (auto, 1fr, auto) } else { (1fr, auto) },
      column-gutter: 10pt,
      align: horizon,
      ..if logo != none { (image(logo, height: if theme == "stage" { 1.3 * s } else { 3 * s }, fit: "contain"),) },
      [
        #if data.band != "" { text(size: 1.6 * s * k, weight: "bold", data.band); linebreak() }
        #text(size: 1.1 * s * k, weight: "semibold", fill: if theme == "stage" { black } else { rgb("#1d5fa0") }, data.title)
      ],
      align(right, text(size: 0.9 * s * k, fill: muted, place.join("\n"))),
    )
    v(-0.2 * s * k)
    line(length: 100%, stroke: (if theme == "stage" { 1.2pt } else { 0.6pt }) + black)
  }
}

// A line across with a word in the middle (breaks, the encore).
#let divider(word, s, weight: "bold") = grid(
  columns: (1fr, auto, 1fr),
  column-gutter: 0.4em,
  align: horizon,
  line(length: 100%, stroke: 0.8pt + black),
  text(weight: weight, word),
  line(length: 100%, stroke: 0.8pt + black),
)

#let stage(s) = {
  set text(size: s)
  head(s)
  v(0.1 * s)
  let indent = 1.5em
  for e in data.entries {
    if e.kind == "song" {
      // Number, title and key on one baseline; the notes under the title.
      block(above: 0.3 * s, below: 0.3 * s, breakable: false, {
        set par(leading: 0.18 * s, spacing: 0.24 * s)
        box(width: indent, text(size: 0.5 * s, fill: muted, e.no))
        text(weight: "bold", e.title + if e.next != "" { "\u{a0}\u{a0}" + e.next })
        h(1fr)
        text(size: 0.62 * s, weight: "semibold", e.key)
        if e.notes != "" {
          parbreak()
          pad(left: indent, text(size: 0.4 * s, fill: muted, e.notes))
        }
      })
    } else if e.kind == "break" or e.kind == "encore" {
      block(above: 0.4 * s, below: 0.4 * s, text(size: 0.5 * s, divider(upper(e.title) + if e.minutes != "" { "  ·  " + e.minutes }, s)))
    } else {
      block(above: 0.3 * s, below: 0.3 * s, pad(left: indent * 0.5, text(size: 0.5 * s, style: "italic", fill: muted, e.title + if e.notes != "" { " – " + e.notes })))
    }
  }
}

#let technical(s) = {
  set text(size: s)
  head(s)
  v(0.4 * s)
  let cols = data.columns
  let titles = cols.map(c => c.title)
  set table(stroke: 0.4pt + luma(150), inset: (x: 0.35 * s, y: 0.3 * s))
  table(
    columns: cols.map(c => if c.wide > 0 { c.wide * 1fr } else { auto }),
    fill: (x, y) => if y == 0 { luma(232) } else { none },
    table.header(..titles.map(t => text(weight: "bold", size: 0.9 * s, t))),
    ..data.entries.map(e => {
      if e.kind == "song" {
        cols.map(c => {
          let v = e.at(c.key, default: "")
          table.cell(fill: tint(e.color))[#if c.key == "title" { text(weight: "semibold", v + if e.next != "" { "\u{a0}\u{a0}" + e.next }) } else if c.mono { mono(v) } else { v }]
        })
      } else {
        // Specials: the times, then the text across the rest.
        let lead = cols.position(c => c.key == "title")
        let before = cols.slice(0, lead).map(c => {
          let v = if c.key == "no" { "" } else { e.at(c.key, default: "") }
          table.cell(fill: luma(238), mono(v))
        })
        before + (table.cell(colspan: cols.len() - lead, fill: luma(238))[
          #text(weight: "bold", upper(e.kind_label))#if e.title != "" and e.title != e.kind_label [ · #e.title]#if e.length != "" [ · #e.length]#if e.notes != "" [ · #text(style: "italic", e.notes)]
        ],)
      }
    }).flatten(),
  )
  if data.parts.len() > 1 {
    v(0.4 * s)
    text(size: 0.9 * s, fill: muted, data.parts.map(p => p.name + ": " + p.length + " (" + str(p.songs) + ")").join("   ·   "))
  }
}

#let elegant(s) = {
  set text(size: s)
  head(s)
  v(0.5 * s)
  set align(center)
  for e in data.entries {
    if e.kind == "song" {
      block(spacing: 0.45 * s, breakable: false, {
        text(e.title)
        if e.next_label != "" { text(style: "italic", fill: muted, size: 0.7 * s, "  " + lower(e.next_label)) }
      })
    } else if e.kind == "break" or e.kind == "encore" {
      block(spacing: 0.7 * s, text(size: 0.72 * s, tracking: 0.12em, fill: muted, upper(e.title)))
    }
  }
}

#let body(s) = if theme == "stage" { stage(s) } else if theme == "elegant" { elegant(s) } else { technical(s) }

// The largest size that fits the page (halving the range, to half a point).
#layout(size => {
  let fit(s) = measure(block(width: size.width, body(s))).height <= size.height
  let s = range.max
  if not fit(s) {
    let lo = range.min
    let hi = range.max
    while hi - lo > 0.4pt {
      let mid = (lo + hi) / 2
      if fit(mid) { lo = mid } else { hi = mid }
    }
    s = lo
  }
  [#metadata(s / 1pt) <fit-size>]
  body(s)
})
