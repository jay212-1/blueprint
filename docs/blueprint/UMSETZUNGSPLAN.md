# Blueprint – Umsetzungsplan (M0)

## Context

Blueprint ist das neue Planungstool der Showtime Suite: Mikrofonpläne (Theater/Musical), Setlisten, Patchpläne mit mehreren Locations und weitere Pläne, alles druckfertig als PDF. Die Anforderungen stehen in `~/Desktop/blueprint-prompt.md`, die Analyse der Suite in `blueprint/docs/blueprint/ARCHITEKTUR-ANALYSE.md`.

Entschieden (mit dir abgestimmt am 2026-11-19):
- **Druck: Typst eingebettet.** Prototyp: 3-seitiger Mic-Plot mit wiederholtem Tabellenkopf in 0,3 s. Kosten: ca. +35 MB im Programm, erster Build ca. 5 Min. auf dem Intel-Mac.
- **Aufbau: Workspace wie Show Master.**
- **Schriften: Inter, JetBrains Mono, Source Serif** (OFL), ins PDF eingebettet.

Repos: Code in `jay212-1/blueprint` (geklont nach `~/Desktop/dev/blueprint`), Releases in `jay212-1/blueprint-releases`.

## Repo-Aufbau (Muster: Show Master + Lantern)

```
blueprint/
  Cargo.toml            [workspace] members=["crates/*"], [workspace.package] version="0.1.0",
                        edition 2021, rust-version, license "MIT OR Apache-2.0",
                        Lints und Profile wie Lantern
  (kein .cargo/config.toml: RUST_MIN_STACK dort ließ rustc abstürzen, siehe decisions.md)
  crates/bp-core/       Modell, IDs, Projektdatei (JSON, schema_version, Migrationen, atomares Speichern),
                        Undo-Stack, Autosave/Recovery, Prüfungen, Berechnungen, Import (CSV/TSV/XLSX)
  crates/bp-print/      Typst-Welt, Vorlagen (templates/*.typ, getrennt vom Code), Themes, Fonts (fonts/*.ttf),
                        Daten → JSON → Vorlage, PDF (typst-pdf), Vorschau-Rendering (typst-render)
  crates/blueprint/     Binary + egui-App: main.rs, app.rs, app_*.rs je Bereich, theme.rs, table.rs (Tabelleneditor),
                        platform.rs, update.rs
  assets/               icon.svg/.png/.ico
  packaging/            macos/build-dmg.sh + Info.plist (com.cheter.blueprint), windows/blueprint.iss (eigene AppId),
                        linux/blueprint.desktop, build-on-linux.sh
  scripts/              ui.sh und setup.sh (identische Kopien aus Show Master)
  release.sh            Muster Lantern, RELEASES="jay212-1/blueprint-releases"
  install-mac.sh, install.sh, build.rs (Windows-Icon), CHANGELOG.md, CLAUDE.md (Suite-Muster), README.md, .gitignore
  docs/blueprint/       ARCHITEKTUR-ANALYSE.md, FORTSCHRITT.md, decisions.md
```

Kein neues GUI-Framework und kein neues Dateiformat: egui/eframe 0.37 mit dem Suite-Farbschema (Werte aus `show-master/crates/sm-desktop/src/theme.rs`), JSON mit serde.

## Datenmodell (bp-core, Kern)

- `Project { schema_version, meta (Produktion, Venue, Daten), cast, roles, scenes, transmitters, capsules, locations, devices, links, plans: Vec<Plan> }`.
- Stabile IDs (`u64` aus `next_id`, wie Show Master), Pläne verweisen nur per ID (Single Source of Truth).
- `Plan { id, name, revision, changed, printed_snapshot (für Änderungsmarkierung), kind: PlanKind }`. `PlanKind` ist ein Enum: MicPlan, Setlist, Patch, StagePlot, RunOfShow, Contacts, Packlist, Intercom, Table. Neue Plantypen = neue Variante + Editor + Vorlage. Das ist das erweiterbare Plan-System.
- Undo/Redo als Schnappschuss-Stack wie in `show-master/crates/sm-server/src/doc.rs` (begrenzte Tiefe, schnelle Folgen gleicher Änderungen zu einem Schritt zusammengefasst).
- Speichern atomar wie in `show-master/crates/sm-core/src/showfile.rs` (tmp, fsync, rename), Autosave als Recovery-Datei neben dem Projekt, Wiederherstellung beim Start.
- Endung `.blueprint`. Einstellungen in `dirs::config_dir()/blueprint/settings.json`, für Tests per Umgebungsvariable umgelenkt.

## Meilensteine

**M1 – Grundgerüst (dieser nächste Schritt)**
1. Workspace, Lints, Skripte, `CLAUDE.md`, `.gitignore`, CHANGELOG `## 0.1.0`, `build.rs`, App-Icon (Blaupause mit Raster im Suite-Stil, Farbe z. B. `#3b82f6`).
2. Fenster im Suite-Stil: Startseite (Neu, Öffnen, Zuletzt), linke Navigation (Stammdaten, Pläne), Inhaltsbereich.
3. Projekt anlegen/öffnen/speichern, Undo/Redo, Autosave + Recovery.
4. Wiederverwendbarer Tabelleneditor: Tastaturbedienung (Tab, Enter, Pfeile), Mehrfachauswahl, Drag & Drop zum Sortieren, Copy/Paste als TSV (Excel/Numbers), Farbcodierung.
5. Stammdaten-Editoren: Cast, Rollen (inkl. Besetzungen), Szenen, Sender, Kapseln, Locations.
6. `packaging/macos/build-dmg.sh` und `install-mac.sh` lauffähig, `release.sh` vorbereitet (nur Dry-Run).
7. Launcher-Kachel in `showtime-suite/apps.json` (id `blueprint`, Name, Tagline „Planning: mic plots, setlists, patch plans and more.“, Farbe, `releases: jay212-1/blueprint-releases`, Asset-Namen, `dev`-Teil) und Icon `assets/apps/blueprint.png`. **Diff zeige ich dir vor dem Commit**, weil es ein anderes Programm betrifft. Ins Release-Repo des Launchers kopiere ich den Katalog erst mit dem ersten Blueprint-Release.
8. Tests: Modell, Speichern/Laden, Migration (Gerüst), Undo; UI-Tests mit egui_kittest und Screenshots (`BLUEPRINT_SHOTS`).

**M2 – Mikrofonplan:** Matrix Szene × Sender, Ansicht pro Darsteller:in, Tagesbesetzung, Swaps/Shares, Spares mit Schnelltausch, DCA-Belegung pro Szene, Akkuplan, Frequenzliste. Konfliktprüfungen (doppelte Belegung, Rolle ohne Sender, Swap-Fenster zu kurz, defekter Sender / Spare im Plan, doppelte Frequenz). Die Intermodulation ist im Modell vorbereitet. PDFs: Mic-Plot, Mic-Cards, Swap-Liste.

**M3 – Setlisten:** Songbibliothek, Setlisten, Spezialeinträge, Dauern und Uhrzeiten, Themes „Bühne“, „Technik/FOH“, „Elegant“, Auto-Schriftgröße mit Mindestgröße und Warnung.

**M4 – Patch:** Locations, Geräte mit Ports, Verbindungen (Multicore, Dante, AES50, MADI …), Input- und Output-Patch mit Splits und vollem Signalweg, Prüfungen, Kabel- und Stativliste, Etiketten/Scribble-Strips.

**M5 – Sonstige Pläne:** Stageplot (Symbolbibliothek, Raster, Maßstab, Verknüpfung mit Inputs), Run of Show, Kontakte, Packliste, Intercom, freie Tabellen.

**M6 – Import/Export und Release:** CSV/TSV/XLSX (calamine) mit Spaltenzuordnung, Gesamt-PDF mit Inhaltsverzeichnis, CSV, PNG, ein versioniertes Austauschformat für die Suite (Kanalnamen, Patch, DCA), Packaging Linux/Windows, Dry-Run, erster Release in `jay212-1/blueprint-releases` nach deiner Freigabe.

Nach jedem Meilenstein: `docs/blueprint/FORTSCHRITT.md` aktualisieren, Version erhöhen, committen, pushen, und du testest.

## Druck (bp-print)

- `typst` + `typst-pdf` + `typst-render` 0.15 über eine eigene, schlanke `World`-Implementierung (oder `typst-as-lib`): nur die mitgelieferten Fonts, kein Paket-Download, keine Systemschriften. Das hält die PDFs auf jedem Rechner gleich.
- Plan-Daten gehen als JSON in die Vorlage (`sys.inputs` bzw. eine virtuelle Datei). Vorlagen und Themes liegen als `.typ` in `crates/bp-print/templates/` und werden per `include_str!` eingebaut.
- Gemeinsames Layout: Kopf- und Fußzeile (Produktion, Venue, Datum, Revision, Seite x/y), A4/A3/Letter, hoch/quer, Themes Bühne/Technik/Tinte sparen.
- Vorschau in der App: Seiten mit `typst-render` als Textur.
- Golden-Tests: Seitenzahl und Kerninhalte der erzeugten Dokumente.

## Gemeinsame Crates (Vorschlag, erst nach deiner Zustimmung)

Die Suite hat kein gemeinsames Crate. Kandidaten für später: ein Kanal-/Patch-/DCA-Austauschmodell, das Show Master übernehmen könnte. Ich lege es in M4/M6 zuerst in `bp-core` an und schlage dir dann den Umzug vor.

## Verifikation

- `cargo test --workspace` und `cargo clippy --workspace --all-targets -- -D warnings` nach jedem Schritt.
- UI-Tests mit egui_kittest und Screenshots in den Scratchpad, die ich mir ansehe (Startseite, Stammdaten, Tabelleneditor mit Tastatur und Paste).
- Speichern → Absturz simulieren → Recovery-Test, Undo/Redo-Tests.
- `bash install-mac.sh` baut das DMG und installiert lokal. `./release.sh status` und Dry-Run funktionieren, ohne etwas hochzuladen.
- Ende M1: Commit und Push nach `jay212-1/blueprint`. Die Launcher-Änderung zeige ich dir vorher.
