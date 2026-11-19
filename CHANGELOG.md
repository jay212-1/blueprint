# Changelog

## 0.1.0

- First version: the groundwork. Start page (new, open, recent, recovery of an unsaved project),
  navigation for the production data, master data and (later) the plans.
- Project file `.blueprint` (JSON with a schema version), atomic saving with backups, autosave every
  20 seconds and recovery after a crash, undo and redo.
- Master data: production, cast, roles with casting (lead, alternate, cover, swing), scenes,
  transmitters, capsules and locations.
- Table editor: keyboard (arrows, Tab, Enter, typing), multiple selection, reorder by dragging,
  copy and paste with Excel / Numbers (TSV), colours per row, context menu.
- Packaging for macOS (DMG), Windows (Inno Setup) and Linux (tar.gz); `release.sh` with a dry run.
