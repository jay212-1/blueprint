// The cover of a production pack: the production, then the contents with
// the page numbers of the whole PDF.
#import "common.typ": *

#let m = data.meta
#set document(title: (if m.production != "" { m.production } else { "Untitled production" }) + " – Production pack")
#set page(
  paper: data.paper,
  margin: (x: 2cm, top: 2.4cm, bottom: 2cm),
  footer: {
    set text(size: th.small, fill: th.muted)
    grid(columns: (1fr, auto), [#data.revision], [#data.pages pages])
  },
)
#set text(font: "Inter", size: th.size, lang: "en")

#v(3cm)
#text(size: 11pt, tracking: 0.2em, fill: th.accent, weight: "bold")[PRODUCTION PACK]
#v(4pt)
#text(size: 30pt, weight: "bold")[#if m.production != "" { m.production } else { "Untitled production" }]
#v(6pt)
#text(size: 13pt, fill: th.muted)[#(m.company, m.venue, m.dates).filter(x => x != "").join(" · ")]
#if data.mic_revision != "" {
  v(2pt)
  text(size: 10pt, fill: th.muted, data.mic_revision)
}
#v(1.4cm)
#text(size: 14pt, weight: "bold")[Contents]
#v(4pt)
#table(
  columns: (1fr, auto, auto),
  stroke: (x, y) => (bottom: 0.4pt + th.line),
  inset: (x: 4pt, y: 6pt),
  align: (left, left, right),
  ..data.entries.map(e => (
    text(weight: "bold", e.title),
    text(fill: th.muted, size: th.small, if e.kind != e.title { e.kind } else { "" }),
    mono(e.pages),
  )).flatten(),
)
