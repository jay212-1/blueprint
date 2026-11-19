// Scribble strips for the desks and port labels for the stageboxes: rows of
// boxes to cut out, channel number small, name bold. Data: bp-print/src/data.rs.
#import "common.typ": *
#show: page-setup

#let cut = (paint: luma(120), thickness: 0.5pt, dash: "dashed")

#if data.strips.len() == 0 [
  Nothing to label yet: put inputs on desk channels and stagebox ports.
]

#for (k, s) in data.strips.enumerate() {
  if k > 0 { v(10pt) }
  let rows = calc.ceil(s.cells.len() / s.per)
  for r in range(rows) {
    let cells = s.cells.slice(r * s.per, calc.min((r + 1) * s.per, s.cells.len()))
    // The title stays with the first row.
    block(breakable: false, below: 6pt, {
      if r == 0 { block(below: 4pt, text(size: th.size * 1.1, weight: "bold", s.title)) }
      grid(
      columns: (1fr,) * s.per,
      rows: 1.35cm,
      stroke: cut,
      ..cells.map(c => box(
        width: 100%,
        height: 100%,
        fill: tint(c.color),
        inset: 3pt,
        {
          text(size: th.small, fill: th.muted, font: "JetBrains Mono", c.no)
          v(1fr)
          align(center, text(size: th.size * 1.15, weight: "bold", hyphenate: false, c.name))
          v(1fr)
        },
      )),
    )})
  }
}
