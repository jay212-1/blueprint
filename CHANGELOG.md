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
- Mic plan: matrix scene × transmitter with colours per performer, the usual transmitter per role
  (plus a B transmitter for A/B miking) and exceptions typed straight into the matrix (role or
  performer names), who is on stage per scene.
- Performers: capsule, B capsule and position per person, the transmitters they wear over the show.
- Swaps worked out from the plan with the time there is (from scene start and length), where and
  who swaps; casts of the day (double casting, cover and swing shows) change who wears what.
- Spares: "Swap in the spare" moves a broken transmitter's whole plan to a spare and marks it
  defect.
- DCA programming per scene ("Label: Role, Role", desk channels follow from the plan), battery
  plan, frequency list by band.
- Checks: transmitter on two people at once, role on stage without a mic or without a performer,
  one person in two roles at once, swap too short (configurable), spare or defect transmitter in
  the plan, duplicate frequencies, optional third order intermodulation. Problems are framed red in
  the tables and listed under Checks with a jump to the place.
- Printing with embedded Typst and its own fonts (Inter, JetBrains Mono, Source Serif): mic plot
  (wide plans split over pages, scene columns repeated), mic cards to cut out, swap list, frequency
  list, DCA sheet, battery plan. A4, A3 and Letter, portrait or landscape, themes Stage, Technical
  and Save ink, head and foot with production, venue, revision and page numbers. Preview in the app,
  PDF export.
- Revisions: "Issue Rev N" stamps the prints; the mic plot marks what changed since the revision
  before.
- Demo project "The Lighthouse Keeper" (start page): 12 performers, 15 scenes, 16 transmitters with
  2 spares, swaps, A/B miking, two casts of the day.
- Scenes: a column for the roles on stage; capsules can be marked as spares.
- Setlists: a song library (title, key, tempo, length, lead vocal, line-up, light and sound cue,
  colour, notes) and any number of setlists that use its songs, so a change shows everywhere.
  Titles typed into a setlist go into the library. Special entries: break, announcement,
  changeover, encore. Transitions gap, attacca and segue; start time and gap per setlist give the
  time of day of every entry, set lengths, music time, total and end. Duplicate a setlist (e.g. for
  a shorter festival set), a logo per setlist.
- Setlist prints on one page with the type as large as fits (warning when it does not fit even at
  the smallest size): themes Stage (big titles and keys for the floor), Technical / FOH (times,
  tempo, line-up, cues) and Elegant (serif, centred).
- Demo band show "Northbound Static" (start page): 16 songs, a club show and a festival set.
- Patch: devices at the locations (stageboxes, I/O racks, consoles, splitters, amps, recorders,
  IEM transmitters) with their port counts and connectors, lines between them (multicore, Dante,
  AES50, MADI, AVB, AES/EBU) with channel count and cable label.
- Input patch: channel, name, source, mic or DI, stand, 48V, player, stagebox and port, line and
  line channel (an analog multicore uses the port; a stagebox with one line uses it), one column
  per desk and recorder for splits, and the whole signal path. New rows follow on from the row
  above; known condenser mics switch 48V on.
- Output patch: desk bus or matrix, line, device and port, where it goes (PA, monitor, IEM,
  recording, broadcast), for whom and the mix notes.
- Patch checks: a port or desk channel used twice, two signals on one line channel, a port, desk
  channel or line channel above the count, a full line, an input that reaches no desk, an output
  that goes nowhere, a condenser mic without 48V. Listed under Checks with the mic plan.
- Patch prints: input list, a page per stagebox to tape on, output list, monitor / IEM list per
  musician, cables and stands counted from the patch as a packing list, scribble strips for the
  desks and port labels for the stageboxes.
- The demo band show has a full patch: 26 inputs on two stageboxes, split to FOH, monitors and the
  recording, 12 outputs.
