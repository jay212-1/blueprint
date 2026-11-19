# Blueprint – notes for Claude Code

Planning tool of the Showtime Suite: mic plans (theatre / musicals), setlists, patch plans across
several locations and more, all printable as PDF. Rust + egui, built like the other suite apps
(Show Master, Lantern).

## Git workflow

- **Work directly on `main`.** No feature branches, no pull requests.
- **Before you start and before every push:** `git pull --rebase`, then build and test, then push.
- Never force-push `main`, never rewrite pushed history, never delete or move tags.

## Version and changelog with every change

- Every change raises the version in the root `Cargo.toml` (`[workspace.package] version`), then
  `cargo update -w` so `Cargo.lock` follows. Small changes: patch step; bigger features: minor step.
- Add a `## x.y.z` section at the top of `CHANGELOG.md` (English).
- New decisions go into `docs/blueprint/decisions.md` (German, one line each).
- After each milestone: update `docs/blueprint/FORTSCHRITT.md`.

## Releases

Released files go to **`jay212-1/blueprint-releases`** (only downloads and a README, never develop
there). `./release.sh` publishes (Arch PC: Linux + Windows, Mac: the DMG), `./release.sh status`
shows what is up, `./release.sh dry-run` builds this computer's parts and uploads nothing.

## The other projects

All live side by side in `~/Desktop/dev` (folder names in lower case):

| Folder | Repository | Release downloads |
|---|---|---|
| `show-master` | Cheter0410/Show-Master | Cheter0410/show-master-releases |
| `lantern` | Cheter0410/lantern | Cheter0410/lantern-releases |
| `showtime-suite` | Cheter0410/showtime-suite | Cheter0410/showtime-suite-releases |
| `beamweave` | Cheter0410/beamweave | Cheter0410/beamweave-releases |
| `blueprint` | jay212-1/blueprint | jay212-1/blueprint-releases |

Showtime Suite installs, updates and starts the others; its catalog is `showtime-suite/apps.json`.

## Layout

- `crates/bp-core` – model (stable `u64` ids, plans refer to ids only), project file (JSON with
  `schema_version` and migrations, atomic save, backups, autosave / recovery), undo stack, tables
  (`sheet.rs`: columns, TSV copy / paste), times and numbers.
- `crates/bp-core/src/mic.rs` – the mic plan (usual transmitter per role, exceptions per scene,
  casts of the day, swaps, DCAs, batteries) and `MicView`, which works out the matrix, swaps and
  problems; `mic_grid.rs` shows it as tables; `grid.rs` is the table interface all lists share;
  `demo.rs` the demo musical and the demo band show. `setlist.rs` – song library, setlists, times
  (`timing`) and their tables. `patch.rs` – devices, lines, inputs with splits, outputs, signal
  paths, checks, cable and stand list; `patch_grid.rs` its tables. `plans.rs` – the other plans
  (run of show, contacts, packing list, intercom, free tables), `stage.rs` – the stage plot and its
  symbols as shapes, `plan_grid.rs` their tables. `import.rs` – reading CSV / TSV / spreadsheets
  into any table and CSV export; `exchange.rs` – the suite exchange file
  (`docs/blueprint/exchange-format.md`).
- `crates/bp-print` – Typst printing: `templates/*.typ` (layout and themes, kept apart from the
  code; `setlist.typ` fits the type to one page, `plan.typ` the other plans and the stage plot, `package.typ` the cover of the production pack), `fonts/` (OFL, embedded), `data.rs` (plans to JSON for the templates).
- `crates/blueprint` – the egui app: `app.rs` (window, project actions), `app_*.rs` (pages; `app_stage.rs` the stage plot editor, `app_import.rs` import / CSV export, `app_start.rs` the start screen and the new-project form, `open_files*.rs` files handed over by macOS),
  `table.rs` (reusable spreadsheet editor that emits `Action`s), `theme.rs` (suite colours),
  `settings.rs` (`BLUEPRINT_CONFIG_DIR` redirects them; tests never touch the real settings).
- `packaging/`, `release.sh`, `install-mac.sh`, `install.sh` – as in Lantern.
- `scripts/setup.sh` and `scripts/ui.sh` are identical copies of the files in Show Master, Lantern and
  Showtime Suite – change all of them together.

## Build and test

```bash
cargo test --workspace
BLUEPRINT_SHOTS=/some/folder cargo test --workspace   # UI tests also write screenshots
cargo clippy --workspace --all-targets -- -D warnings
```

- `install-mac.sh` / `install.sh` build with `--profile local` (Cargo.toml: no LTO, incremental,
  seconds per change); `release.sh` and the packaging scripts use `release` (thin LTO, minutes).
  `PROFILE=release` switches the install scripts to the release build.
- No `unwrap()` / `expect()` / `panic!` in code that runs on user input or files (Clippy warns;
  tests may).
- UI texts are English. Only glyphs of egui's default fonts in UI strings (others panic in tests).
