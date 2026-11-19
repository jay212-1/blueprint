# Entscheidungen

- 2026-11-19: Druck mit eingebettetem Typst (typst, typst-pdf, typst-render 0.15), nur mitgelieferte Schriften, kein Paket-Download.
- 2026-11-19: Aufbau als Workspace wie Show Master: `bp-core` (Modell, Datei, Undo), später `bp-print` (Typst), `blueprint` (App).
- 2026-11-19: Schriften Inter, JetBrains Mono und Source Serif (OFL), werden ins PDF eingebettet.
- 2026-11-19: Projektdatei `.blueprint` als JSON mit `schema_version`, stabile `u64`-IDs, Pläne verweisen nur per ID.
- 2026-11-19: Undo als Schnappschuss-Stack (200 Schritte, Tippen in derselben Zelle innerhalb 1,2 s ist ein Schritt).
- 2026-11-19: Der Tabelleneditor zeichnet nur und liefert Aktionen; die App wendet sie mit Undo an (ein Editor für alle Tabellen).
- 2026-11-19: Kein `RUST_MIN_STACK` in `.cargo/config.toml`: damit stürzte rustc beim Bauen von wgpu/LLVM ab. Im Typst-Prototyp war die Variable nur für den LTO-Link nötig; für M2 prüfen und ggf. nur im Release-Bau setzen.
- 2026-11-19: Release-Repo `jay212-1/blueprint-releases`, Bundle-ID `com.cheter.blueprint`, Windows-AppId `{152A3A39-AF82-47BD-A7FF-4376CC77D835}`.
- 2026-11-19: Nur Zeichen aus egui-Standardschriften in UI-Texten (z. B. kein „▾“; der Pfeil wird gezeichnet).
