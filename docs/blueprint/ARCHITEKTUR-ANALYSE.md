# Architektur-Analyse der Showtime Suite (für Blueprint)

Stand: 2026-11-19. Untersucht: `show-master`, `lantern`, `beamweave`, `showtime-suite` in `~/Desktop/dev`.

## Repositories und Ordner

| Ordner | Code-Repo | Release-Repo | Aufbau |
|---|---|---|---|
| `show-master` | Cheter0410/Show-Master | Cheter0410/show-master-releases | Workspace, `crates/*` (sm-core, sm-desktop, …) + Web-UI |
| `lantern` | Cheter0410/lantern | Cheter0410/lantern-releases | Ein Crate in `src/` + kopierte Crates in `crates/` |
| `beamweave` | Cheter0410/beamweave | Cheter0410/beamweave-releases | Ein Crate in `src/` |
| `showtime-suite` | Cheter0410/showtime-suite | Cheter0410/showtime-suite-releases | Ein Crate in `src/` (der Launcher) |
| `blueprint` (neu) | **jay212-1/blueprint** | **jay212-1/blueprint-releases** | – |

- Jedes Programm ist ein **eigenes Repo**, Ordnernamen klein geschrieben, alle nebeneinander in `~/Desktop/dev`.
- Das Release-Repo enthält nur die Downloads und eine README. Entwickelt wird nie dort.
- Git: direkt auf `main`, `git pull --rebase` vor Arbeit und vor jedem Push, kein Force-Push, Tags `vX.Y.Z` sind heilig.
- Jede Änderung: Version in `Cargo.toml` hoch, `cargo update -w`, Eintrag `## x.y.z` oben in `CHANGELOG.md` (Englisch).
- Jedes Repo hat eine `CLAUDE.md` mit denselben Abschnitten (Who works here, Git workflow, Version and changelog, Never commit, Releases, The other projects, Build and test, Project notes).

## Gemeinsame Crates

Es gibt **kein gemeinsames Crate-Repo**. Geteilt wird durch **Kopien**: `sm-timecode`, `sm-rtpmidi`, `sm-mdns` liegen in Show Master und als Kopie in `lantern/crates`. Eine Korrektur gehört in beide (Hinweis in der Commit-Nachricht). Ebenso werden Skripte als identische Kopien gepflegt:

- `scripts/ui.sh`: gemeinsames Terminal-Aussehen der Build-, Install- und Release-Skripte (in Show Master, Lantern und Showtime Suite identisch).
- `scripts/setup.sh`: Installationshelfer (Rust, Pull, App-Ordner) – identisch in Show Master, Lantern und Showtime Suite.

Theme, Settings und Logging sind **pro Programm** implementiert, nach demselben Muster.

## GUI

- **egui / eframe 0.37**, `default-features = false`, Features `wgpu`, `default_fonts`, `x11`, `wayland`.
- Fenster: `ViewportBuilder` mit Titel = Anzeigename, `with_app_id("<binary>")`, ca. 1400×860, Mindestgröße, Drag & Drop, Icon aus `assets/icon.png` per `include_bytes!`.
- Theme: dunkel, Konstanten im Code (`theme.rs` bzw. oben in `app.rs`):
  Hintergrund ≈ `rgb(12–17, 13–19, 16–24)`, Panels ≈ `rgb(20–24, 22–27, 27–34)`, Linien `rgb(44,48,58)`, Text `rgb(232,235,239)`, gedämpft `rgb(138–140,144–146,158–160)`.
  Bedeutungsfarben überall gleich: Grün `#22c55e` (OK/Next), Blau `#3b82f6` (aktiv), Amber `#f59e0b`/`#ffc45c`, Rot `#ef4444` (Fehler), Edit `#2563eb`, Live `#dc2626`.
  Show Master: `theme::apply(ctx, high_contrast, touch)` setzt `Visuals::dark()` mit diesen Farben, Abstände, Schriftgrößen (14/11.5/20/13.5) und optional Touch-Größen.
- Schriften: egui-Standardschriften, keine eigenen Font-Dateien.
- Icons: keine Icon-Font. Symbole werden mit dem Painter gezeichnet oder als Text/Unicode.
- UI-Sprache: **Englisch**, schlicht. Dokumentation und Skriptausgaben sind Deutsch.
- UI-Tests mit **egui_kittest** (headless, mit `wgpu` für Screenshots über eine Umgebungsvariable wie `LANTERN_SHOTS`). Tests nutzen einen eigenen Settings-Ordner und fassen nie die echten Einstellungen an.

## Modulaufbau

- Ein-Crate-Programme (Lantern, Beamweave, Launcher): `main.rs` (Argumente, `--version`, Fenster), `app.rs` + `app_*.rs` (eine Datei pro Bereich der Oberfläche, z. B. `app_start.rs`, `app_log.rs`, `app_tests.rs`), Fachmodule ohne UI (`show.rs`, `deck.rs`, `timecode.rs` …), `platform.rs` für Betriebssystem-Spezielles.
- Show Master trennt stärker: `sm-core` (Modell, Showdatei, Logik, rein ohne I/O), `sm-desktop` (egui), `showmaster` (Binary). Abhängigkeiten zeigen nur nach unten.

## Dateien, Formate, Speicherorte

- **Serialisierung: serde + JSON** (`serde_json`, pretty). Kein RON/TOML für Daten.
- Show Master: Showdatei `.smshow` mit `schema_version`, Migrationen als Funktionen auf `serde_json::Value` (`migrate_v1_to_v2` …, eine pro Schritt), danach typisiert geparst. Felder mit `#[serde(default)]`, leere Felder mit `skip_serializing_if`.
- **Atomares Speichern**: in `.<name>.tmp-<pid>` schreiben, `fsync`, `rename`. Ein Absturz hinterlässt nie eine halbe Datei.
- **Autosave und Wiederherstellung**: eine Wiederherstellungskopie neben der Datei, beim Start angeboten (`recoverable`, `Recover { accept }`).
- **Undo/Redo**: Show Master speichert ganze Dokument-Schnappschüsse in einer `VecDeque` (begrenzte Zahl Schritte); schnelle Folgen gleicher Änderungen (Fader) werden zu einem Schritt zusammengefasst.
- Einstellungen: `dirs::config_dir()/<programm>/settings.json` (mit Umgebungsvariable für Tests).

## Fehler, Logging, Lints, Tests

- `thiserror` für Fehlertypen der Logik, `tracing` (Show Master) bzw. ein eigenes Log (Lantern).
- Workspace-Lints (Lantern, Show Master): `unsafe_code = "deny"`, Clippy `unwrap_used`, `expect_used`, `panic` = warn. `cargo clippy --all-targets -- -D warnings` muss sauber sein. Kein `rustfmt.toml` (Standardformat).
- `rust-version = "1.85"`, Edition 2021.
- Profile: release `opt-level=3`, `lto="thin"`, `codegen-units=4`, `strip=true`; dev `debug="line-tables-only"`, Abhängigkeiten mit `opt-level=2`.
- Lizenz: `license = "MIT OR Apache-2.0"` in `Cargo.toml`, keine Lizenzdatei im Repo.

## Build, Packaging, Release

- `build.rs`: bettet unter Windows das Icon ein (`winresource`, `assets/icon.ico`).
- `assets/`: `icon.svg` (Quelle, 1024er Kachel, abgerundetes Quadrat `#14171c` mit Rand `#2a2f38` und einem farbigen Motiv in der App-Farbe), daraus `icon.png` und `icon.ico` (`node scripts/render-icon.mjs` im Launcher).
- `packaging/macos/build-dmg.sh` + `Info.plist` (Bundle-ID `com.cheter.<binary>`, Kategorie, Mindest-macOS 11): baut `dist/<App>-<v>.dmg`, Ad-hoc-Signatur (keine Notarisierung), nur neu, wenn sich etwas geändert hat (`ui_fingerprint`), schreibt `dist/files-mac.txt`.
- `packaging/build-on-linux.sh`: auf dem Arch-PC Linux-`tar.gz` und Windows-Installer (Inno Setup unter Wine, `packaging/windows/<bin>.iss` mit fester `AppId`, Installation nach `{autopf}\Showtime Suite\<App>`), schreibt `dist/files-linux-pc.txt`.
- `packaging/linux/<bin>.desktop`.
- `install-mac.sh` (Pull, Bauen, nach „Programme › Showtime Suite“ installieren) und `install.sh` (Linux, ohne sudo).
- `release.sh` (Deutsch kommentiert): ein Befehl auf jedem Rechner; `./release.sh status`. Mac lädt das DMG hoch, der Arch-PC Linux + Windows. Hat sich seit dem letzten Release etwas geändert, wird die Version erhöht, „What's new“ aus den Commits erzeugt, committet, getaggt (`vX.Y.Z`), gepusht und per `gh release` in das **Release-Repo** hochgeladen. Fehlende Teile eines Releases werden aus dem Tag nachgebaut. Asset-Namen: `<App>-<v>.dmg`, `<App>-Setup-<v>.exe`, `<bin>-<v>-linux-x86_64.tar.gz`.
- Kein CI für Releases; gebaut wird lokal auf Mac und Arch-PC.

## Launcher (Showtime Suite)

- Katalog `showtime-suite/apps.json`: pro App `id`, `name`, `tagline`, `color`, `releases` (Release-Repo), `macos` (`asset_suffix`, `app`), `windows` (`asset_prefix`, `asset_suffix`, `inno_app_id`, `exe`), `linux` (`asset_suffix`, `bin`), `dev` (`repo`, `folder`, `release`, `install_macos`, `install_linux`).
- Der Katalog wird zusätzlich in den Default-Branch von `Cheter0410/showtime-suite-releases` kopiert; installierte Launcher lesen ihn von dort. Eine neue App erscheint so ohne neuen Launcher.
- Icons: eingebaut in `assets/apps/<id>.png` (`catalog.rs`), sonst `icon_url` (PNG im Release-Repo des Launchers, `icons/<id>.png`).
- Der Launcher findet Releases über die GitHub-API (`/repos/<releases>/releases`) und nimmt das neueste Release mit einer Datei für das eigene System.
- Programme prüfen selbst auf Updates (`update.rs`, GitHub-API des Release-Repos).

## Folgerungen für Blueprint

- Rust + egui/eframe 0.37 mit demselben Theme-Muster, UI Englisch, JSON-Projektdatei mit `schema_version` und Migrationen, atomares Speichern, Autosave mit Wiederherstellung, Undo per Schnappschuss.
- Binary `blueprint`, Anzeigename „Blueprint“, Bundle-ID `com.cheter.blueprint`, Assets `Blueprint-<v>.dmg`, `Blueprint-Setup-<v>.exe`, `blueprint-<v>-linux-x86_64.tar.gz`.
- Skripte (`release.sh`, `install-mac.sh`, `install.sh`, `packaging/…`) nach dem Lantern-Muster, `scripts/ui.sh` und `scripts/setup.sh` als identische Kopien.
- Besonderheit: Code- und Release-Repo gehören **jay212-1**, nicht Cheter0410. `release.sh` und `apps.json` zeigen auf `jay212-1/blueprint-releases`; Cheter braucht Schreibrecht dort, wenn er vom Arch-PC Linux/Windows hochladen soll.
- Neu in der Suite: ein Druck-Modul (PDF). Lantern nutzt bereits `hayro`, um PDFs zu rendern – das wäre für die Druckvorschau wiederverwendbar.
