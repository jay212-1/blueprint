// Mic plot: scenes down, transmitters across. Wide plans are split into
// blocks of transmitters; scene number and title repeat on every block.
#import "common.typ": *
#show: page-setup

#set text(hyphenate: true)
#for (bi, block) in data.blocks.enumerate() {
  if bi > 0 { pagebreak() }
  table(
    columns: (auto, 1.6fr) + (1fr,) * block.columns.len(),
    table.header(
      repeat: true,
      [No.], [Scene],
      ..block.columns.map(c => align(center)[#c]),
    ),
    ..block.rows.map(r => (
      mono(r.no),
      [#r.title #if r.warn != "" { text(fill: red, size: th.small)[\ #r.warn] }],
      ..r.cells.map(c => table.cell(
        fill: tint(c.color),
        stroke: if c.changed { 1.4pt + black } else { 0.4pt + th.line },
        align: center + horizon,
      )[
        #set text(size: 0.92em)
        #if c.swap { text(fill: th.muted)[→ ] }#if c.changed { strong(c.text) } else { c.text }
        #if c.who != "" { linebreak(); text(size: th.small, fill: th.muted, c.who) }
      ]),
    )).flatten(),
  )
}
