# Blueprint

Planning for live shows, part of the Showtime Suite: mic plots for theatre and musicals, setlists,
patch plans across several locations and more – everything printable as a clean PDF. Written in Rust
(egui), for macOS, Windows and Linux.

**Status: 0.1 – the groundwork.** Projects, master data and the table editor work; the plans and the
PDF output come in the next versions (see `docs/blueprint/UMSETZUNGSPLAN.md`).

- **One project file** (`.blueprint`) per production: production data, cast, roles with their
  casting (lead, alternate, cover, swing), scenes, transmitters, capsules and locations. Plans refer
  to these, so a name is changed in one place.
- **Tables like a spreadsheet:** arrow keys, Tab and Enter, typing starts editing, several rows
  selected with Shift / Cmd, rows reordered by dragging the row number, copy and paste with Excel or
  Numbers, colours per row.
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
