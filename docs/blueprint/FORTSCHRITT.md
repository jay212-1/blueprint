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

## M2 – Mikrofonplan (in 0.1.0, 2026-11-19) ✅

- Matrix Szene × Sender mit Farben pro Darsteller:in; Rollen- oder Personennamen direkt in die
  Zelle tippen (Ausnahme für diese Szene), üblicher Sender pro Rolle, B-Sender (A/B-Miking).
- Darsteller:innen: Kapsel, B-Kapsel, Position, welche Sender sie über die Show tragen.
- Swaps automatisch aus dem Plan, mit Zeitfenster aus Szenenstart und -länge; Ort und wer tauscht.
- Tagesbesetzung (Alternates, Cover, Swing-Show): wählbar oben, alle Ansichten und Drucke folgen.
- Spare-Schnelltausch („Swap in the spare“): Sender raus, Spare übernimmt überall, Sender = defekt.
- DCA-Belegung pro Szene („Label: Rolle, Rolle“, Pultkanäle werden berechnet), Akkuplan,
  Frequenzliste je Band.
- Prüfungen: Sender doppelt belegt, Rolle ohne Mikro, Rolle ohne Darsteller:in, Person in zwei
  Rollen gleichzeitig, Swap zu kurz (Mindestzeit einstellbar), Spare/defekt im Plan, doppelte
  Frequenz, optional Intermodulation 3. Ordnung. Rote Rahmen in den Tabellen, Liste unter
  „Checks“ mit Sprung an die Stelle.
- Druck mit Typst (eigene Schriften): Mic-Plot, Mic-Cards zum Ausschneiden, Swap-Liste,
  Frequenzliste, DCA-Sheet, Akkuplan; A4/A3/Letter, hoch/quer, Themes Stage/Technical/Save ink;
  Vorschau in der App, PDF-Export; Revisionen mit Änderungsmarkierung.
- Demo-Musical „The Lighthouse Keeper“ (Startseite).
- Tests: 27 Kern-Tests, 3 Druck-Tests (alle Dokumente in allen Themes), 10 UI-Tests.

**Zum Testen:** Startseite › „Try the demo musical“. Dann: Mic plot ansehen (Zellen mit der Maus
überfahren), in eine Zelle eine Rolle tippen, „Transmitters per role“ › „Swap in the spare“,
„Cast of the day“ umschalten, „Print / PDF“ › Dokumente und Themes durchklicken, PDF exportieren,
„Issue Rev 1“, etwas ändern, „Issue Rev 2“ und die Markierung im Mic-Plot ansehen.

**Offen / bewusst später:** Export der DCA-Belegung an die Live-Tools (Austauschformat, M6),
Gesamt-PDF (M6), Akkuplan pro Vorstellung mit Zeiten (nur Regeln: vor der Show, Pause, nach
Szene).

## M3 – Setlisten (in 0.1.0, 2026-11-19) ✅

- Songbibliothek: Titel, Tonart, BPM, Länge, Lead-Vocal, Besetzung, Licht- und Ton-Cue, Farbe,
  Notizen; „Used“ zeigt, wie oft ein Song in Setlisten steht.
- Beliebig viele Setlisten (Navigation › SETLISTS, „+ New setlist“), die auf die Bibliothek
  verweisen: Änderungen an einem Song gelten überall. Neue Titel in einer Setliste landen in der
  Bibliothek. Duplizieren (z. B. Festival-Kurzset), Löschen mit Undo.
- Spezialeinträge: Break, Announcement, Changeover, Encore; Übergänge Gap / Attacca / Segue.
- Startzeit und Lücke pro Setliste: Startzeit und Uhrzeit jedes Eintrags, Set-Längen, Musikzeit,
  Gesamtdauer, Ende; Warnung bei Songs ohne Länge.
- Logo pro Setliste (PNG, JPEG, SVG).
- Druck auf einer Seite mit automatischer Schriftgröße (Warnung, wenn es auch bei der kleinsten
  nicht passt): Stage (große Titel, Tonart, Übergangszeichen), Technical / FOH (alle Spalten mit
  Zeiten und Cues, leere Spalten fallen weg), Elegant (Serif, zentriert, für Gäste).
- Demo-Band „Northbound Static“ (Startseite): 16 Songs, Club-Show in zwei Sets mit Zugabe und
  ein 45-Minuten-Festivalset.
- Tests: 35 Kern-Tests, 5 Druck-Tests, 11 UI-Tests.

**Zum Testen:** Startseite › „Try the demo band show“. Club show ansehen, „Details“ aufklappen und
Startzeit/Lücke ändern, in die Titelspalte einen neuen Song tippen (erscheint in der Song library),
Übergang auf Attacca setzen, „Print…“ › Themes durchklicken, Logo wählen, PDF exportieren. Eine
Setliste duplizieren und kürzen; im Druck wird die Schrift größer.

**Offen / bewusst später:** Import von Songlisten aus CSV/XLSX mit Spaltenzuordnung (M6),
Verknüpfung von Songs mit Patch/Inputs (M4).

## M4 – Patch (in 0.1.0, 2026-11-19) ✅

- Geräte pro Location (Stagebox, I/O-Rack, Pult, Splitter, Amp, Recorder, IEM) mit Port-Zahlen
  und Steckertyp; Leitungen (Multicore, Dante, AES50, MADI, AVB, AES/EBU) mit Kanalzahl, Länge und
  Kabelbezeichnung.
- Input-Patch: Kanal, Name, Quelle, Mikro/DI, Stativ, 48V, Musiker:in, Stagebox/Port, Leitung und
  Leitungskanal (automatisch aus Stagebox und Port), eine Spalte pro Pult/Recorder für Splits,
  ganzer Signalweg in der letzten Spalte. Neue Zeilen zählen Kanal und Port weiter.
- Output-Patch: Pult-Bus/Matrix → Leitung → Gerät/Port → Ziel (PA, Monitor, IEM, Recording,
  Broadcast), für wen, Mix-Wünsche.
- Prüfungen: Port/Pultkanal doppelt, Leitungskanal doppelt, Port-/Kanalzahl überschritten,
  Leitung voll, Input ohne Pult, Output ohne Ziel, Kondensator ohne 48V. Unter „Checks“ mit Sprung.
- Drucke: Input-Liste, Stagebox-Patch (eine Seite pro Box zum Ankleben), Output-Liste,
  Monitor/IEM-Liste pro Musiker:in, Kabel- und Stativliste als Packliste, Scribble-Strips für die
  Pulte und Port-Etiketten für die Stageboxen.
- Demo-Band mit vollem Patch: 26 Inputs auf zwei Stageboxen (Multicore und Dante), Split auf FOH,
  MON und REC, 12 Outputs mit Wedges und IEMs.
- Tests: 43 Kern-Tests, 6 Druck-Tests, 12 UI-Tests.

**Zum Testen:** Startseite › „Try the demo band show“ › PATCH › „Input patch“. Einen Port
doppelt belegen (z. B. Snare top auf Port 1) und unter „Checks“ ansehen, Mikro „KM184“ eintippen
(48V geht an), unter „Devices“ ein Gerät hinzufügen (neue Location wird angelegt), „Cables and
stands“ ansehen, „Print / PDF“ › PATCH › alle sechs Drucke durchklicken.

**Offen / bewusst später:** Verknüpfung Stageplot ↔ Inputs (M5), Packliste mit Ersatz und
manuellen Posten (M5), Austauschformat für die Live-Tools (Kanalnamen, Patch, DCA) und Vorschlag
für ein gemeinsames Crate (M6).

## M5 – Sonstige Pläne (in 0.1.0, 2026-11-19) ✅

- Erweiterbares Plan-System: `Project.plans` mit `PlanKind` (Stageplot, Run of Show, Kontakte,
  Packliste, Intercom/Funk, freie Tabelle). In der Navigation unter „MORE PLANS“, neu über
  „+ New plan“; umbenennen, Notizen, duplizieren, löschen (mit Undo).
- Stageplot: Bühne in Metern mit Raster und Vorderkante, 16 Symbole (Stative, Wedges, Drumkit,
  Keys, Amps, DI, Strom, Podest, Person, IEM, Speaker, Stagebox, Stuhl, Text). Klicken wählt, Ziehen
  verschiebt (rastet ein), Pfeile schieben, R dreht, Entf löscht; Eigenschaftsleiste mit Label,
  Inputs („1-7, 9“), Drehen, Duplizieren. Verknüpfte Inputs als Kanal-Badge. Druck quer mit
  Maßstab und Liste „Inputs on stage“, dazu PNG-Export (300 dpi).
- Run of Show: Start, Länge, Ende (berechnet; ohne Start folgt die Zeile der vorigen), Was, Wer,
  Wo, Cue, Notizen, Farbe.
- Kontakte mit Gruppen (im Druck getrennt), „Add the cast“ übernimmt die Besetzung.
- Packliste: „Count from patch and mic plan“ zählt Leitungen, Kabel, DIs, Mikros, Stative, Sender,
  Kapseln und Akkus; eigene Zeilen und Gepackt-Häkchen bleiben beim Neuzählen erhalten.
- Intercom/Funk: Kanäle (Partyline, Wireless, Funk, Programm) und Personen mit Talk/Listen pro
  Kanal.
- Freie Tabellen mit Spalten der Art Text, Zahl, Uhrzeit, Dauer, Checkbox, Auswahl, Farbe; im
  Druck mit gezeichneten Kästchen.
- Alle Pläne drucken in den drei Themes; die Demo-Band hat je einen Plan jeder Art.
- Tests: 56 Kern-Tests, 7 Druck-Tests, 15 UI-Tests.

**Zum Testen:** Startseite › „Try the demo band show“ › MORE PLANS › „Stage plot“. Ein Stativ
ziehen, R drücken, im Feld „Inputs“ „25“ eintragen, ein Symbol aus „Add“ hinzufügen, „Print…“ ›
„Export PNG…“. Danach „Packing list“ › „Count from patch and mic plan“, beim Intercom den Reiter
„Channels“, bei der Guest list unter „Columns“ eine Spalte vom Typ „Check box“ anlegen.

**Offen / bewusst später:** Symbole frei skalieren per Maus (geht über Breite/Tiefe in der
Tabelle), eigene Symbole/Bilder, Mehrfachauswahl auf der Bühne.

## M6 – Import/Export (in 0.1.0, 2026-11-19) ✅, Release offen

- Import in jede Tabelle (Stammdaten, Patch, Songs, Setlisten, alle Pläne): CSV, TSV, Text
  (UTF-8, UTF-16, Windows-1252; Trennzeichen wird erkannt) und Tabellen (XLSX, XLS, ODS, mit
  Blattwahl). Spalten werden über die Titel zugeordnet (auch deutsche wie „Kürzel“, „Notizen“,
  „Farbe“, „Kanal“) und lassen sich vor dem Import ändern; Vorschau der ersten Zeilen. Was nicht
  passt, wird pro Zeile und Spalte gemeldet; ein Undo nimmt den ganzen Import zurück.
- „Export CSV…“ über jeder Tabelle (mit BOM für Excel).
- Production pack: Deckblatt mit Inhaltsverzeichnis und den gewählten Drucken in einem PDF, jeder
  mit seinem eigenen Papier und Theme. Standard: alles mit Inhalt außer Mic-Cards und Etiketten.
- Stageplot als PNG (aus M5).
- Austauschdatei für die Live-Tools (Production › „Export exchange file…“, `.showtime.json`):
  Kanalnamen, Patch, Mikrofonplan pro Szene, DCAs. Beschrieben in `exchange-format.md`.
- Windows-Installer verknüpft `.blueprint`-Dateien.
- Probelauf `./release.sh dry-run` auf dem Mac: Programm (x86_64, 9 min) und DMG gebaut,
  `Blueprint-0.1.0.dmg` 25 MB; Windows und Linux baut der Arch-PC.
- Tests: 64 Kern-Tests, 8 Druck-Tests, 17 UI-Tests.

**Zum Testen:** Eine Excel- oder CSV-Liste (z. B. Input-Liste mit „Kanal, Name, Mikro, Phantom“)
über „Import…“ im Input patch laden, die Zuordnung prüfen, importieren, Cmd+Z. „Print / PDF“ ›
„Production pack“: Häkchen setzen, PDF exportieren. Production › „Export exchange file…“.

**Offen:** erster Release nach deiner Freigabe (Linux und Windows baut der Arch-PC), danach der
Launcher-Katalog im Release-Repo der Suite. Ob das Austauschformat in ein gemeinsames Crate der
Suite wandert (damit Show Master es direkt liest), entscheiden wir noch.

## 0.2.0 – Startbildschirm und Dateiverwaltung (2026-11-19) ✅

- Startbildschirm wie bei Show Master und Lantern: Hintergrund mit Blaupausen-Raster und
  leuchtendem Bühnenplan, Logo, Karten „New project“ / „Open project“ (und „Recover“), Liste der
  zuletzt geöffneten Projekte mit Ordner und Datum, Demos unten rechts in der Ecke.
- „New project“: Name und Ort (Standard Dokumente/Blueprint), legt einen eigenen Ordner mit der
  Projektdatei an und speichert sofort.
- File-Menü: Start screen, New project…, Open…, Open recent, Save, Save as…, Close project.
- `.blueprint`-Dateien öffnen per Drag & Drop aufs Fenster und aus dem Finder bei laufendem Programm.
- Ab jetzt steigt die Version mit jeder Änderung.
- Tests: 18 UI-Tests (neu: Formular, Ordner, Recent, File-Menü, Drag & Drop).

## 0.2.1 – Stimmlage und On/Off stage im Mic Plot (2026-11-19) ✅

- Cast hat die Spalte „Vocal range“ (Import erkennt „Stimmlage“, „Range“, „Voice“), gedruckt auf den Mic-Cards.
- Mic Plot: kräftige Zellen = auf der Bühne; blasse Zelle mit Farbbalken und Rollenname = hinter der
  Bühne, trägt den Sender weiter (Kanal zu). In der App und im Druck (mit Legende).
- Tests: 65 Kern-Tests (neu: Off-stage-Spuren), 8 Druck-Tests, 18 UI-Tests.
