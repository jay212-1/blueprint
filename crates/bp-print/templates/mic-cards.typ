// One card per performer: transmitters, capsule, position, swaps, notes.
// Cards do not break across pages; the dashed frame is the cut line.
#import "common.typ": *
#show: page-setup

#let card(c) = block(
  breakable: false,
  width: 100%,
  height: data.card_height * 1cm,
  inset: 9pt,
  stroke: (paint: luma(140), thickness: 0.5pt, dash: "dashed"),
)[
  #grid(
    columns: (1fr, auto),
    text(size: th.size * 1.5, weight: "bold")[#c.name],
    text(size: th.size * 1.2, fill: th.muted, weight: "bold")[#c.short],
  )
  #v(-2pt)
  #text(fill: th.muted)[#c.roles]
  #v(2pt)
  #if c.txs.len() > 0 {
    table(
      columns: (auto, auto, 1fr),
      table.header([TX], [Capsule], [Scenes]),
      ..c.txs.map(t => (mono(t.tx), [#t.capsule], [#t.scenes])).flatten(),
    )
  } else [_No transmitter in the plan._]
  #if c.position != "" [*Position:* #c.position \ ]
  #for s in c.swaps [→ #s \ ]
  #if c.notes != "" [#text(size: th.small)[#c.notes]]
]

#grid(
  columns: (1fr,) * data.card_columns,
  column-gutter: 0pt,
  row-gutter: 0pt,
  ..data.cards.map(card),
)
