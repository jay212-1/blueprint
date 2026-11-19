// Any list as a table: swap list, frequencies, DCA sheet, battery plan.
#import "common.typ": *
#show: page-setup

#for (i, sec) in data.sections.enumerate() {
  if i > 0 and sec.at("newpage", default: false) { pagebreak() }
  if sec.title != "" {
    v(if i > 0 and not sec.at("newpage", default: false) { 10pt } else { 0pt })
    text(size: th.size * 1.15, weight: "bold")[#sec.title]
    v(2pt)
  }
  table(
    columns: data.widths.map(w => if w == 0 { auto } else { w * 1fr }),
    fill: (x, y) => if y > 0 and th.fill and calc.even(y) { th.zebra } else { none },
    table.header(repeat: true, ..data.head.map(h => [#h])),
    ..sec.rows.map(r => r.cells.enumerate().map(((j, c)) => table.cell(
      fill: if r.warn and th.fill { rgb("#fde2e0") } else { tint(r.at("color", default: "")) },
    )[#if j == 0 { mono(c) } else if r.warn and j == r.cells.len() - 1 { text(fill: red)[#c] } else { c }])).flatten(),
  )
}
