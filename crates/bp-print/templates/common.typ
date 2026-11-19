// Shared layout of every Blueprint print: page, head and foot, themes.
// The data comes in as JSON (`sys.inputs.data`), see bp-print/src/data.rs.

#let data = json(bytes(sys.inputs.data))

// Themes: "stage" (big, high contrast), "technical" (compact), "ink" (black
// and white, no fills).
#let themes = (
  stage: (size: 12pt, small: 9pt, head: 1.5em, inset: 5pt, fill: true, line: black, muted: luma(70), accent: black, zebra: luma(240)),
  technical: (size: 8.5pt, small: 7pt, head: 1.35em, inset: 3pt, fill: true, line: luma(150), muted: luma(100), accent: rgb("#1d5fa0"), zebra: luma(246)),
  ink: (size: 9pt, small: 7.5pt, head: 1.35em, inset: 3.5pt, fill: false, line: black, muted: luma(60), accent: black, zebra: white),
)
#let th = themes.at(data.theme, default: themes.technical)

// Colour names of the project to light print tints.
#let tints = (
  red: rgb("#f8c9c4"), orange: rgb("#fbd9b0"), yellow: rgb("#f8eba0"),
  green: rgb("#c9ebc4"), blue: rgb("#c4dcf5"), purple: rgb("#dcccf0"),
)
#let tint(name) = if th.fill and name != "" { tints.at(name, default: none) } else { none }

#let mono(body) = text(font: "JetBrains Mono", size: 0.95em, body)

#let page-setup(body) = {
  let m = data.meta
  set document(title: data.title + if m.production != "" { " – " + m.production } else { "" })
  set page(
    paper: data.paper,
    flipped: data.landscape,
    margin: (x: 1.3cm, top: 2.1cm, bottom: 1.6cm),
    header: {
      set text(size: th.small, fill: th.muted)
      grid(
        columns: (1fr, auto),
        align: (left + bottom, right + bottom),
        [
          #text(size: th.size * 1.25, weight: "bold", fill: black)[#if m.production != "" { m.production } else { "Untitled production" }]
          #if m.venue != "" [ · #m.venue]
        ],
        [
          #text(size: th.size * 1.25, weight: "bold", fill: th.accent)[#data.title]
          #if data.subtitle != "" [\ #data.subtitle]
        ],
      )
      v(-4pt)
      line(length: 100%, stroke: 0.6pt + th.line)
    },
    footer: context {
      set text(size: th.small, fill: th.muted)
      grid(
        columns: (1fr, auto, 1fr),
        align: (left, center, right),
        [#(m.company, m.dates).filter(x => x != "").join(" · ")],
        [#data.revision],
        [Page #counter(page).display() / #counter(page).final().first()],
      )
    },
  )
  set text(font: "Inter", size: th.size, lang: "en")
  set par(leading: 0.45em)
  set table(stroke: 0.4pt + th.line, inset: th.inset)
  show table.cell.where(y: 0): set text(weight: "bold")
  body
  if data.legend != "" {
    v(6pt)
    text(size: th.small, fill: th.muted, data.legend)
  }
}
