//! UI tests: start page, master data tables (keyboard, paste, undo), saving
//! and recovery. `BLUEPRINT_SHOTS=<folder>` saves pictures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use blueprint::{App, Page};
use bp_core::mic_grid::MicSheet;
use bp_core::sheet::Sheet;
use egui::{Event, Key, Modifiers};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

/// All tests share one settings folder (never the real one).
fn config() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap().keep();
        std::env::set_var("BLUEPRINT_CONFIG_DIR", &dir);
        dir
    })
}

fn harness() -> Harness<'static, App> {
    config();
    let mut app = App::new(&egui::Context::default(), None);
    app.dialogs = false;
    app.untitled_recovery = None;
    let mut b = Harness::builder().with_size(egui::vec2(1280.0, 800.0));
    if std::env::var_os("BLUEPRINT_SHOTS").is_some() {
        b = b.wgpu();
    }
    let mut h = b.build_ui_state(
        |ui, app: &mut App| {
            app.logic(ui.ctx());
            app.ui_root(ui);
        },
        app,
    );
    settle(&mut h);
    h
}

fn settle(h: &mut Harness<'static, App>) {
    for _ in 0..6 {
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn shot(h: &mut Harness<'static, App>, name: &str) {
    let Some(dir) = std::env::var_os("BLUEPRINT_SHOTS").map(PathBuf::from) else {
        return;
    };
    let _ = std::fs::create_dir_all(&dir);
    match h.render() {
        Ok(img) => {
            let _ = img.save(dir.join(format!("{name}.png")));
        }
        Err(e) => eprintln!("no picture of {name}: {e}"),
    }
}

fn key(h: &mut Harness<'static, App>, key: Key, modifiers: Modifiers) {
    h.key_press_modifiers(modifiers, key);
    settle(h);
}

fn type_text(h: &mut Harness<'static, App>, text: &str) {
    h.input_mut().events.push(Event::Text(text.into()));
    settle(h);
}

fn open_sheet(h: &mut Harness<'static, App>, sheet: Sheet) {
    h.state_mut().page = Page::Sheet(sheet);
    settle(h);
}

fn project<'a>(h: &'a Harness<'static, App>) -> &'a bp_core::model::Project {
    &h.state().doc.as_ref().unwrap().project
}

#[test]
fn start_page_creates_a_project() {
    let mut h = harness();
    shot(&mut h, "start");
    assert!(h.state().doc.is_none());
    h.get_by_label("New project").click();
    settle(&mut h);
    assert!(h.state().doc.is_some());
    shot(&mut h, "production");
}

#[test]
fn typing_in_the_cast_table_with_tab_and_enter() {
    let mut h = harness();
    h.get_by_label("New project").click();
    settle(&mut h);
    open_sheet(&mut h, Sheet::Cast);
    h.get_by_label("Add person").click();
    settle(&mut h);
    // A new row starts editing its first cell.
    type_text(&mut h, "Anna Berg");
    key(&mut h, Key::Tab, Modifiers::NONE);
    // Typing on a selected cell starts editing it.
    type_text(&mut h, "AB");
    key(&mut h, Key::Enter, Modifiers::NONE);
    let p = project(&h);
    assert_eq!(p.cast.len(), 1);
    assert_eq!(p.cast[0].name, "Anna Berg");
    assert_eq!(p.cast[0].short, "AB");
    assert!(h.state().doc.as_ref().unwrap().dirty);

    // Undo goes back cell by cell.
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(project(&h).cast[0].short, "");
    key(&mut h, Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(project(&h).cast[0].short, "AB");
    shot(&mut h, "cast");
}

#[test]
fn pasting_rows_from_a_spreadsheet() {
    let mut h = harness();
    h.get_by_label("New project").click();
    settle(&mut h);
    open_sheet(&mut h, Sheet::Transmitters);
    // Focus the empty table, then paste three rows.
    h.get_by_label_contains("No transmitters yet").click();
    settle(&mut h);
    h.input_mut()
        .events
        .push(Event::Paste("1\tTX 1\n2\tTX 2\n3\tTX 3".into()));
    settle(&mut h);
    let names: Vec<_> = project(&h).transmitters.iter().map(|t| t.name.clone()).collect();
    assert_eq!(names, ["TX 1", "TX 2", "TX 3"]);
    shot(&mut h, "transmitters");

    // One undo step removes the whole paste.
    h.get_by_label("Undo").click();
    settle(&mut h);
    assert!(project(&h).transmitters.is_empty());
}

#[test]
fn roles_link_to_the_cast() {
    let mut h = harness();
    h.get_by_label("New project").click();
    settle(&mut h);
    open_sheet(&mut h, Sheet::Cast);
    h.get_by_label_contains("No cast yet").click();
    settle(&mut h);
    h.input_mut()
        .events
        .push(Event::Paste("Anna Berg\tAB\nTom Kurz\tTK".into()));
    settle(&mut h);
    open_sheet(&mut h, Sheet::Roles);
    h.get_by_label_contains("No roles yet").click();
    settle(&mut h);
    h.input_mut()
        .events
        .push(Event::Paste("Mara\tAnna Berg\tTK (cover)".into()));
    settle(&mut h);
    let p = project(&h);
    assert_eq!(p.roles.len(), 1);
    assert_eq!(p.roles[0].casting.len(), 2);
    assert_eq!(p.roles[0].lead(), p.person_named("Anna Berg").map(|x| x.id));
    shot(&mut h, "roles");
}

#[test]
fn save_reopen_and_recover() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Show.blueprint");
    let mut h = harness();
    h.get_by_label("New project").click();
    settle(&mut h);
    h.state_mut()
        .doc
        .as_mut()
        .unwrap()
        .edit("meta:production", |p| p.meta.production = "Harbour Lights".into());
    assert!(h.state_mut().save_to(Some(&path)));
    assert!(!h.state().doc.as_ref().unwrap().dirty);

    // A newer autosave (the app crashed) is offered on opening.
    let mut crashed = project(&h).clone();
    crashed.meta.venue = "Stadttheater".into();
    bp_core::project::autosave(&crashed, &path).unwrap();
    drop(h);

    let mut h = harness();
    h.state_mut().open_path(&path);
    settle(&mut h);
    assert!(h.state().modal_open());
    shot(&mut h, "recover");
    h.get_by_label("Recover").click();
    settle(&mut h);
    let p = project(&h);
    assert_eq!(p.meta.production, "Harbour Lights");
    assert_eq!(p.meta.venue, "Stadttheater");
}

fn demo(h: &mut Harness<'static, App>) {
    h.get_by_label_contains("Try the demo musical").click();
    settle(h);
    assert_eq!(h.state().page, Page::Mic(MicSheet::Plot));
}

#[test]
fn demo_mic_plot_and_checks() {
    let mut h = harness();
    demo(&mut h);
    shot(&mut h, "mic-plot");
    // The demo has one intended problem: a swap that is too tight.
    h.get_by_label_contains("Checks: 1 problem").click();
    settle(&mut h);
    h.get_by_label_contains("only 0:25");
    shot(&mut h, "checks");
    h.get_by_label("Show").click();
    settle(&mut h);
    assert_eq!(h.state().page, Page::Mic(MicSheet::Plot));
    let st = &h.state().tables["mic:Plot"];
    assert_eq!(st.cursor.0, 2, "scene 1.3");
}

#[test]
fn typing_a_role_into_the_mic_plot() {
    let mut h = harness();
    demo(&mut h);
    // Scene 2.3 (row 10), transmitter 13 (column 2 + 12): put Mara there.
    {
        let st = h.state_mut().tables.entry("mic:Plot".into()).or_default();
        st.select(&[9]);
        st.cursor = (9, 14);
    }
    settle(&mut h);
    type_text(&mut h, "Mara");
    key(&mut h, Key::Enter, Modifiers::NONE);
    let p = project(&h);
    let mara = p.roles[0].id;
    let scene = p.scenes[9].id;
    assert_eq!(p.mic.tx_in(scene, mara), Some(p.transmitters[12].id));
    assert!(p.scenes[9].roles.contains(&mara), "Mara is now on stage in 2.3");
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert!(!project(&h).scenes[9].roles.contains(&mara));
}

#[test]
fn a_spare_takes_over() {
    let mut h = harness();
    demo(&mut h);
    h.state_mut().page = Page::Mic(MicSheet::Tracks);
    settle(&mut h);
    shot(&mut h, "tracks");
    h.get_by_label("Swap in the spare").click();
    settle(&mut h);
    let p = project(&h);
    // The first active transmitter (1) is out, the first spare (S1) takes over.
    assert_eq!(p.transmitters[0].status, bp_core::model::TxStatus::Defect);
    assert_eq!(p.mic.track(p.roles[0].id).and_then(|t| t.tx), Some(p.transmitters[14].id));
}

#[test]
fn cast_of_the_day() {
    let mut h = harness();
    demo(&mut h);
    h.state_mut().page = Page::Mic(MicSheet::Performers);
    settle(&mut h);
    let cast_b = project(&h).mic.variants[0].id;
    h.state_mut().doc.as_mut().unwrap().edit("t", |p| p.mic.variant = Some(cast_b));
    settle(&mut h);
    shot(&mut h, "performers-cast-b");
    let v = h.state_mut().mic_view();
    let mia = project(&h).cast[6].id;
    assert!(!v.person_txs(mia).is_empty(), "Mia wears Mara's transmitter in Cast B");
}

#[test]
fn print_preview_and_pdf() {
    let mut h = harness();
    demo(&mut h);
    h.state_mut().page = Page::Print;
    settle(&mut h);
    h.get_by_label_contains("page").hover();
    shot(&mut h, "print-mic-plot");
    h.get_by_label("Mic cards").click();
    settle(&mut h);
    shot(&mut h, "print-mic-cards");
    h.get_by_label("Issue Rev 1").click();
    settle(&mut h);
    assert_eq!(project(&h).mic.revision.number, 1);
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("plot.pdf");
    assert!(h.state_mut().export_pdf(bp_print::Doc::MicPlot, &pdf));
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
}

#[test]
fn band_show_setlists() {
    let mut h = harness();
    h.get_by_label_contains("demo band show").click();
    settle(&mut h);
    let club = project(&h).setlists[0].id;
    assert_eq!(h.state().page, Page::Setlist(club));
    h.get_by_label_contains("16 songs").hover();
    shot(&mut h, "setlist-club");

    // A new title typed into the setlist lands in the library.
    let songs = project(&h).songs.len();
    {
        let st = h.state_mut().tables.entry(format!("setlist:{club}")).or_default();
        st.select(&[2]);
        st.cursor = (2, bp_core::setlist::setcol::TITLE);
    }
    settle(&mut h);
    type_text(&mut h, "Brand New Song");
    key(&mut h, Key::Enter, Modifiers::NONE);
    let p = project(&h);
    assert_eq!(p.songs.len(), songs + 1, "a new song (the old one is used twice)");
    assert!(p.song_named("Brand New Song").is_some());
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(project(&h).songs.len(), songs);

    // The library shows where songs are used.
    // (The nav is taller than the test window now: go there directly.)
    h.state_mut().page = Page::Songs;
    settle(&mut h);
    shot(&mut h, "song-library");

    // A new setlist, a logo, then the print preview.
    h.state_mut().new_setlist();
    settle(&mut h);
    let p = project(&h);
    assert_eq!(p.setlists.len(), 3);
    let new = p.setlists[2].id;
    assert_eq!(h.state().page, Page::Setlist(new));
    assert_eq!(p.setlists[2].band, "Northbound Static");
    let dir = tempfile::tempdir().unwrap();
    let logo = dir.path().join("logo.svg");
    std::fs::write(&logo, br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#333"/></svg>"##).unwrap();
    assert!(h.state_mut().set_logo(club, &logo));
    let bad = dir.path().join("logo.txt");
    std::fs::write(&bad, "hello").unwrap();
    assert!(!h.state_mut().set_logo(club, &bad));

    h.state_mut().page = Page::Setlist(club);
    settle(&mut h);
    h.get_by_label("Print…").click();
    settle(&mut h);
    assert_eq!(h.state().page, Page::Print);
    assert_eq!(h.state().print.doc, bp_print::Doc::Setlist(club));
    h.get_by_label_contains("type fitted at").hover();
    shot(&mut h, "print-setlist");
    h.get_by_label("Technical / FOH").click();
    settle(&mut h);
    let pdf = dir.path().join("set.pdf");
    assert!(h.state_mut().export_pdf(bp_print::Doc::Setlist(club), &pdf));
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));

    // Deleting a setlist, and undoing it.
    h.state_mut().page = Page::Setlist(new);
    settle(&mut h);
    h.get_by_label("Delete setlist").click();
    settle(&mut h);
    assert_eq!(project(&h).setlists.len(), 2);
    assert_eq!(h.state().page, Page::Songs);
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(project(&h).setlists.len(), 3);
}

#[test]
fn patch_pages_and_checks() {
    use bp_core::patch_grid::PatchSheet;
    let mut h = harness();
    h.get_by_label_contains("demo band show").click();
    settle(&mut h);
    h.get_by_label("Input patch").click();
    settle(&mut h);
    assert_eq!(h.state().page, Page::Patch(PatchSheet::Inputs));
    shot(&mut h, "patch-inputs");

    // Snare top (row 2) onto the kick's port: a clash, shown under Checks.
    {
        let st = h.state_mut().tables.entry("patch:Inputs".into()).or_default();
        st.select(&[2]);
        st.cursor = (2, 8);
    }
    settle(&mut h);
    type_text(&mut h, "1");
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(project(&h).patch.inputs[2].port, Some(1));
    h.get_by_label("Show checks").click();
    settle(&mut h);
    assert_eq!(h.state().page, Page::Checks);
    h.get_by_label_contains("SB1 port 1 has two inputs").hover();
    shot(&mut h, "patch-checks");
    h.get_by_label("Show").click();
    settle(&mut h);
    assert_eq!(h.state().page, Page::Patch(PatchSheet::Inputs), "one problem: the port");
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(project(&h).patch.inputs[2].port, Some(3));

    for s in [PatchSheet::Outputs, PatchSheet::Devices, PatchSheet::Lines, PatchSheet::Pack] {
        h.state_mut().page = Page::Patch(s);
        settle(&mut h);
        shot(&mut h, &format!("patch-{s:?}"));
    }
    h.state_mut().page = Page::Print;
    settle(&mut h);
    h.get_by_label("Stagebox patch").click();
    settle(&mut h);
    h.get_by_label_contains("page").hover();
    shot(&mut h, "print-stagebox");
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("labels.pdf");
    assert!(h.state_mut().export_pdf(bp_print::Doc::Labels, &pdf));
}

#[test]
fn choice_menu_button_stays_when_the_pointer_moves_onto_it() {
    use bp_core::patch_grid::PatchSheet;
    let mut h = harness();
    h.get_by_label_contains("demo band show").click();
    settle(&mut h);
    h.state_mut().page = Page::Patch(PatchSheet::Inputs);
    settle(&mut h);
    let arrows = |h: &Harness<'static, App>| h.query_all_by_label("  ").count();
    let base = arrows(&h);
    // The "Stand" cell of row 3 (x 688..756, rows 28 px from y 157).
    let cell = egui::pos2(720.0, 171.0 + 28.0 * 3.0);
    h.input_mut().events.push(Event::PointerMoved(cell));
    settle(&mut h);
    assert_eq!(arrows(&h), base + 1, "hovering a choice cell shows its menu button");
    // Onto the arrow at the right end of the cell.
    h.input_mut().events.push(Event::PointerMoved(egui::pos2(745.0, cell.y)));
    for _ in 0..4 {
        h.step();
        assert_eq!(arrows(&h), base + 1, "the button must not blink away");
    }
    // Open the menu, move away: it stays open where it was.
    h.get_by_label("  ").click();
    settle(&mut h);
    assert!(h.query_by_label("Tall").is_some(), "the menu is open");
    h.input_mut().events.push(Event::PointerMoved(egui::pos2(900.0, 600.0)));
    settle(&mut h);
    assert!(h.query_by_label("Tall").is_some(), "still open after the pointer left the cell");
    h.get_by_label("Tall").click();
    settle(&mut h);
    assert_eq!(project(&h).patch.inputs[3].stand, bp_core::patch::Stand::Tall);
}

#[test]
fn stage_plot_editor() {
    use bp_core::plans::PlanKind;
    let mut h = harness();
    h.get_by_label_contains("demo band show").click();
    settle(&mut h);
    // The nav is taller than the test window: go there directly.
    let plan = project(&h).plans[0].id;
    h.state_mut().page = Page::Plan(plan);
    settle(&mut h);
    assert_eq!(h.state().page, Page::Plan(plan));
    shot(&mut h, "plan-stage");
    let item = |h: &Harness<'static, App>, label: &str| {
        let PlanKind::StagePlot(s) = &project(h).plans[0].kind else { panic!() };
        s.items.iter().find(|i| i.label == label).cloned()
    };
    let spare = item(&h, "Spare vox").unwrap();
    let (origin, scale) = h.state().stage.view.unwrap();
    let at = |x: f64, y: f64| egui::pos2(origin.x + x as f32 * scale, origin.y + y as f32 * scale);
    // Drag the spare vocal one metre (and a bit) to the left: it snaps to the grid.
    let from = at(spare.x, spare.y);
    let to = at(spare.x - 1.1, spare.y);
    h.input_mut().events.push(Event::PointerMoved(from));
    h.step();
    h.input_mut().events.push(Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    for k in 1..=6 {
        let t = k as f32 / 6.0;
        h.input_mut().events.push(Event::PointerMoved(from + (to - from) * t));
        h.step();
    }
    h.input_mut().events.push(Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    settle(&mut h);
    let moved = item(&h, "Spare vox").unwrap();
    assert!((moved.x - 9.5).abs() < 1e-6, "10.6 - 1.1 snapped to 0.5 m: {}", moved.x);
    assert!((moved.y - 6.5).abs() < 1e-6, "y snapped too: {}", moved.y);
    assert_eq!(h.state().stage.selected, Some((plan, spare.id)));
    shot(&mut h, "plan-stage-selected");
    // R turns it, Delete removes it, undo brings it back.
    key(&mut h, Key::R, Modifiers::NONE);
    assert_eq!(item(&h, "Spare vox").unwrap().rot, 90.0);
    key(&mut h, Key::Delete, Modifiers::NONE);
    assert!(item(&h, "Spare vox").is_none());
    key(&mut h, Key::Z, Modifiers::COMMAND);
    settle(&mut h);
    assert!(item(&h, "Spare vox").is_some());
    // A new symbol from the palette is selected at once.
    h.get_by_label("Chair").click();
    settle(&mut h);
    let PlanKind::StagePlot(s) = &project(&h).plans[0].kind else { panic!() };
    let chair = s.items.last().unwrap();
    assert_eq!(chair.symbol, bp_core::stage::Symbol::Chair);
    assert_eq!(h.state().stage.selected, Some((plan, chair.id)));

    // Print it, and as a picture.
    h.get_by_label("Print…").click();
    settle(&mut h);
    assert_eq!(h.state().print.doc, bp_print::Doc::Plan(plan));
    h.get_by_label_contains("page").hover();
    shot(&mut h, "print-stage");
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("stage.png");
    assert!(h.state_mut().export_png(bp_print::Doc::Plan(plan), &png));
    assert!(std::fs::read(&png).unwrap().starts_with(b"\x89PNG"));
}

#[test]
fn other_plans() {
    let mut h = harness();
    h.get_by_label_contains("demo band show").click();
    settle(&mut h);
    let plans: Vec<(bp_core::model::Id, String)> = project(&h).plans.iter().map(|p| (p.id, p.name.clone())).collect();
    for (id, name) in &plans[1..] {
        h.state_mut().page = Page::Plan(*id);
        settle(&mut h);
        shot(&mut h, &format!("plan-{}", name.replace([' ', '/'], "-")));
    }
    // The intercom's other table.
    h.state_mut().page = Page::Plan(plans[4].0);
    settle(&mut h);
    h.get_by_label("Channels").click();
    settle(&mut h);
    shot(&mut h, "plan-intercom-channels");
    // A new plan.
    h.state_mut().new_plan(bp_core::plans::PlanType::RunOfShow);
    settle(&mut h);
    let n = project(&h).plans.len();
    assert_eq!(n, 7);
    let id = project(&h).plans[n - 1].id;
    assert_eq!(h.state().page, Page::Plan(id));
    h.get_by_label("Delete plan").click();
    settle(&mut h);
    assert_eq!(project(&h).plans.len(), 6);
    assert_eq!(h.state().page, Page::Production);
}

#[test]
fn import_rows_and_export_csv() {
    let mut h = harness();
    h.get_by_label("New project").click();
    settle(&mut h);
    open_sheet(&mut h, Sheet::Cast);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("cast.csv");
    // Excel in Germany: semicolons, Windows-1252.
    std::fs::write(&file, b"Name;K\xfcrzel;Notizen;Farbe\r\nJ\xf6rg Weiss;JW;Ensemble;blue\r\nAnna Berg;AB;Lead;pink\r\n").unwrap();
    assert!(h.state_mut().start_import(&Sheet::Cast, &file));
    settle(&mut h);
    shot(&mut h, "import-cast");
    // German titles are matched too; "Contact" is not in the file.
    {
        let st = h.state_mut().import.as_mut().unwrap();
        assert!(st.header);
        assert_eq!(st.map, vec![Some(0), Some(1), Some(3), Some(4)]);
    }
    settle(&mut h);
    h.get_by_label("Import 2 rows").click();
    settle(&mut h);
    assert!(h.state().import.is_none());
    let cast = &project(&h).cast;
    assert_eq!(cast.len(), 2);
    assert_eq!(cast[0].name, "Jörg Weiss");
    assert_eq!(cast[0].short, "JW");
    assert_eq!(cast[0].notes, "Ensemble");
    // "pink" is no colour: the row is there, the value is reported.
    let msg = h.state().message_text().unwrap_or_default().to_string();
    assert!(msg.contains("Row 3, Colour"), "{msg}");
    shot(&mut h, "import-report");
    h.get_by_label("OK").click();
    settle(&mut h);
    // One undo step takes the whole import back.
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert!(project(&h).cast.is_empty());
    key(&mut h, Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(project(&h).cast.len(), 2);

    let csv = dir.path().join("out.csv");
    assert!(h.state_mut().export_csv(&Sheet::Cast, &csv));
    let text = std::fs::read_to_string(&csv).unwrap();
    assert!(text.starts_with("\u{feff}Name,"), "{text}");
    assert!(text.contains("Jörg Weiss,JW"));
}

#[test]
fn production_pack_and_exchange_file() {
    use bp_print::Doc;
    let mut h = harness();
    h.get_by_label_contains("demo band show").click();
    settle(&mut h);
    h.state_mut().page = Page::Print;
    settle(&mut h);
    h.get_by_label("Production pack").click();
    settle(&mut h);
    assert_eq!(h.state().print.doc, Doc::Package);
    shot(&mut h, "print-pack");
    let all = {
        let s = h.state();
        s.pack(&s.doc.as_ref().unwrap().project).len()
    };
    // Leave the guest list out.
    h.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Guest list").click();
    settle(&mut h);
    let s = h.state();
    let pack = s.pack(&s.doc.as_ref().unwrap().project);
    assert_eq!(pack.len(), all - 1);
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("pack.pdf");
    assert!(h.state_mut().export_pdf(Doc::Package, &pdf));
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));

    let file = dir.path().join("band.showtime.json");
    assert!(h.state_mut().export_exchange(&file));
    let x = bp_core::exchange::read(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(x.production.name, "Northbound Static");
    assert_eq!(x.inputs.len(), 26);
    h.state_mut().page = Page::Production;
    settle(&mut h);
    shot(&mut h, "production-exchange");
    assert!(h.query_by_label("Export exchange file…").is_some());
}
