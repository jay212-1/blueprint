# Fortschritt

## M1 – Grundgerüst (0.1.0, 2026-11-19) ✅

- Workspace (`bp-core`, `blueprint`), Lints, Skripte, Icon (Blaupause mit Raster).
- Startseite (Neu, Öffnen, Zuletzt, Wiederherstellen), linke Navigation, Produktionsdaten.
- Projekt anlegen / öffnen / speichern, Backups, Undo/Redo, Autosave alle 20 s, Recovery.
- Tabelleneditor: Tastatur, Mehrfachauswahl, Drag & Drop, TSV-Copy/Paste, Farben, Kontextmenü.
- Stammdaten: Cast, Rollen (mit Besetzungen), Szenen, Sender, Kapseln, Locations.
- Packaging macOS / Windows / Linux, `release.sh` mit `dry-run`.
- Launcher-Kachel in Showtime Suite (nach Freigabe).
- Tests: 13 Kern-Tests, 5 UI-Tests mit Screenshots.

**Zum Testen:** `bash install-mac.sh` oder `cargo run -p blueprint`. Neues Projekt, Cast und Rollen
eintragen (auch per Copy/Paste aus Excel/Numbers), speichern, Programm hart beenden, wieder öffnen.

## Als Nächstes: M2 – Mikrofonplan

Matrix Szene × Sender, Ansicht pro Darsteller:in, Tagesbesetzung, Swaps/Shares, Spares, DCA,
Akkuplan, Frequenzliste, Konfliktprüfungen, erste PDFs (Mic-Plot, Mic-Cards, Swap-Liste) mit Typst.
