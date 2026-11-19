# Blueprint

Planning for live shows, part of the Showtime Suite: mic plots for theatre and musicals, setlists,
patch plans across several locations and more – everything printable as a clean PDF. Written in Rust
(egui), for macOS, Windows and Linux.

**Status: 0.1, in progress.** Projects, master data, the mic plan, setlists, the patch and their prints work;
the other plans follow (see `docs/blueprint/UMSETZUNGSPLAN.md`).

- **One project file** (`.blueprint`) per production: production data, cast, roles with their
  casting (lead, alternate, cover, swing), scenes, transmitters, capsules and locations. Plans refer
  to these, so a name is changed in one place.
- **Tables like a spreadsheet:** arrow keys, Tab and Enter, typing starts editing, several rows
  selected with Shift / Cmd, rows reordered by dragging the row number, copy and paste with Excel or
  Numbers, colours per row.
- **Mic plan:** scene × transmitter matrix, usual transmitter per role with exceptions per scene,
  A/B miking, swaps with the time there is, casts of the day, spares swapped in with one click, DCA
  programming per scene, battery plan, frequencies, and checks for every clash.
- **Setlists:** a song library and any number of setlists that use it, breaks, announcements,
  changeovers and encores, attacca and segue, times of day from the start time, set lengths, a
  logo. Printed on one page with the type as large as fits, for the stage, for FOH or elegant.
- **Patch:** devices at the locations, multicores and network lines, input patch with splits to
  several desks and the whole signal path, output patch with monitors and IEMs, checks for every
  clash, cables and stands counted from it, input list, stagebox pages, scribble strips.
- **Prints:** mic plot, mic cards, swap list, frequency list, DCA sheet and battery plan as PDF
  (Typst with embedded fonts), A4 / A3 / Letter, themes for the stage, the tech desk or to save
  ink, revisions with change marks. Try the demo musical or the demo band show from the start page.
- **Safe:** undo / redo, atomic saving with backups, autosave every 20 seconds and recovery after a
  crash.

## Install

The easiest way is Showtime Suite. From the source:

```bash
bash install-mac.sh   # macOS
./install.sh          # Linux
```

## Develop

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p blueprint
```

License: MIT OR Apache-2.0.
