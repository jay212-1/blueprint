//! UI tests: start page, master data tables (keyboard, paste, undo), saving
//! and recovery. `BLUEPRINT_SHOTS=<folder>` saves pictures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use blueprint::{App, Page};
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
        .push(Event::Paste("Elphaba\tAnna Berg\tTK (cover)".into()));
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
        .edit("meta:production", |p| p.meta.production = "Wicked".into());
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
    assert_eq!(p.meta.production, "Wicked");
    assert_eq!(p.meta.venue, "Stadttheater");
}
