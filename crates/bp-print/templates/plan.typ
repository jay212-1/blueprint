// The other plans: tables (run of show, contacts, packing list, intercom,
// free tables) or the stage plot drawn from shapes in metres.
#import "common.typ": *
#show: page-setup

// A drawn checkbox (cells "\u{1}check:1" / "\u{1}check:0").
#let checkbox(on) = box(width: 0.85em, height: 0.85em, stroke: 0.6pt + th.line, baseline: 0.1em, {
  if on {
    place(line(start: (18%, 52%), end: (42%, 78%), stroke: 1.1pt))
    place(line(start: (42%, 78%), end: (85%, 20%), stroke: 1.1pt))
  }
})

#let cell-body(c) = if c == "\u{1}check:1" { checkbox(true) } else if c == "\u{1}check:0" { checkbox(false) } else { c }

#let tables() = for (i, sec) in data.sections.enumerate() {
  if sec.title != "" {
    v(if i > 0 { 10pt } else { 0pt })
    block(sticky: true, below: 4pt, text(size: th.size * 1.15, weight: "bold")[#sec.title])
  }
  if sec.head.len() > 0 {
    table(
      columns: sec.widths.map(w => if w == 0 { auto } else { w * 1fr }),
      fill: (x, y) => {
        let t = if y > 0 { tint(sec.rows.at(y - 1).at("color", default: "")) } else { none }
        if t != none { t } else if y > 0 and th.fill and calc.even(y) { th.zebra } else { none }
      },
      table.header(repeat: true, ..sec.head.map(h => [#h])),
      ..sec.rows.map(r => r.cells.map(c => [#cell-body(c)])).flatten(),
    )
  }
}

// Fills of the stage symbols.
#let shade(fill, tint-name) = {
  let base = (light: luma(232), mid: luma(186), dark: luma(72), accent: th.accent).at(fill, default: none)
  if base == none { return none }
  if not th.fill { return (light: white, mid: luma(215), dark: black, accent: black).at(fill, default: none) }
  let t = tints.at(tint-name, default: none)
  if t != none and fill != "accent" { color.mix((base, 50%), (t.darken(if fill == "dark" { 45% } else { 10% }), 50%)) } else { base }
}

#let stage-plot() = {
  let s = data.stage
  layout(size => {
    // Room for the front label and the scale under the stage.
    let room = size.height - 1.6cm - if s.items.len() > 0 { 0cm } else { 0cm }
    let k = calc.min(size.width / (s.width * 1cm), room / (s.depth * 1cm))
    let m = k * 1cm
    let (sw, sd) = (s.width * m, s.depth * m)
    let pt(x, y) = (x * m, y * m)
    let ink = 0.7pt + black
    block(width: sw, height: sd, breakable: false, {
      place(rect(width: sw, height: sd, stroke: none, fill: if th.fill { luma(250) } else { white }))
      if s.show_grid and s.grid > 0 and s.grid * m > 3pt {
        let g = 0.3pt + luma(205)
        let n = calc.floor(s.width / s.grid - 0.001)
        for i in range(1, n + 1) { place(line(start: pt(i * s.grid, 0), end: pt(i * s.grid, s.depth), stroke: g)) }
        let n = calc.floor(s.depth / s.grid - 0.001)
        for i in range(1, n + 1) { place(line(start: pt(0, i * s.grid), end: pt(s.width, i * s.grid), stroke: g)) }
      }
      place(rect(width: sw, height: sd, stroke: 1.2pt + black))
      for sh in s.shapes {
        if sh.shape == "poly" {
          place(polygon(fill: shade(sh.fill, sh.tint), stroke: ink, ..sh.points.map(((x, y)) => pt(x, y))))
        } else if sh.shape == "circle" {
          let r = sh.r * m
          place(dx: sh.x * m - r, dy: sh.y * m - r,
            circle(radius: r, fill: shade(sh.fill, sh.tint), stroke: if sh.fill == "accent" { none } else { ink }))
        } else if sh.shape == "line" {
          let ps = sh.points.map(((x, y)) => pt(x, y))
          place(curve(stroke: ink, curve.move(ps.first()), ..ps.slice(1).map(p => curve.line(p))))
        } else if sh.shape == "text" {
          let t = text(
            size: calc.max(5pt, sh.size * m),
            weight: if sh.bold { "bold" } else { "regular" },
            fill: if sh.light { white } else { black },
            sh.text,
          )
          context {
            let d = measure(t)
            place(dx: sh.x * m - d.width / 2, dy: sh.y * m - d.height / 2, t)
          }
        }
      }
    })
    v(3pt)
    // The front, then the scale bar (1 m).
    grid(
      columns: (1fr, auto, 1fr),
      align: (left + horizon, center + horizon, right + horizon),
      text(size: th.small, fill: th.muted, s.size),
      text(size: th.size, weight: "bold", tracking: 0.15em, upper(s.front)),
      box({
        box(width: m, height: 4pt, stroke: 0.6pt + black, fill: black)
        h(4pt)
        text(size: th.small)[1 m]
      }),
    )
  })
}

#if data.kind == "stage" {
  stage-plot()
  let items = data.stage.items
  if items.len() > 0 {
    pagebreak(weak: true)
    block(sticky: true, below: 4pt, text(size: th.size * 1.15, weight: "bold")[Inputs on stage])
    table(
      columns: (auto, 1fr, 2fr),
      fill: (x, y) => if y > 0 and th.fill and calc.even(y) { th.zebra } else { none },
      table.header(repeat: true, [Ch], [On stage], [Inputs]),
      ..items.map(i => (mono(i.ch), i.what, i.inputs)).flatten(),
    )
  }
} else {
  tables()
}

#if data.notes != "" {
  v(8pt)
  block(breakable: false, text(size: th.small)[*Notes* \ #data.notes])
}
