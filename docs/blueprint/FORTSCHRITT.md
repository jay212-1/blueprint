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

## Als Nächstes: M3 – Setlisten

Songbibliothek, mehrere Setlisten, Spezialeinträge, Dauern und Uhrzeiten, Themes „Bühne“,
„Technik/FOH“, „Elegant“, automatische Schriftgröße mit Mindestgröße und Warnung.
