//! The production page (title, venue … printed on every plan) and the master
//! data lists (cast, roles, scenes, transmitters, capsules, locations).

use crate::app::App;
use crate::table::{self, Action, TableView};
use crate::theme;
use bp_core::model::Meta;
use bp_core::grid::{self, Grid};
use bp_core::sheet::Sheet;
use egui::RichText;

/// A text field of the production data.
type MetaField = fn(&mut Meta) -> &mut String;

impl App {
    pub(crate) fn production_page(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = self.doc.as_mut() else { return };
        ui.heading("Production");
        ui.label(
            RichText::new("Printed in the head and foot of every plan.").color(theme::MUTED),
        );
        ui.add_space(10.0);
        let meta = doc.project.meta.clone();
        let fields: [(&str, &str, MetaField, bool); 5] = [
            ("production", "Production", |m| &mut m.production, false),
            ("company", "Company", |m| &mut m.company, false),
            ("venue", "Venue", |m| &mut m.venue, false),
            ("dates", "Dates", |m| &mut m.dates, false),
            ("notes", "Notes", |m| &mut m.notes, true),
        ];
        egui::Grid::new("meta")
            .num_columns(2)
            .spacing([14.0, 10.0])
            .show(ui, |ui| {
                for (key, label, get, multi) in fields {
                    ui.label(RichText::new(label).color(theme::MUTED));
                    let mut m = meta.clone();
                    let value = get(&mut m);
                    let before = value.clone();
                    let te = if multi {
                        egui::TextEdit::multiline(value).desired_rows(4)
                    } else {
                        egui::TextEdit::singleline(value)
                    };
                    ui.add(te.desired_width(460.0).id_salt(("meta", key)));
                    if *value != before {
                        let new = value.clone();
                        doc.edit(&format!("meta:{key}"), |p| *get(&mut p.meta) = new);
                    }
                    ui.end_row();
                }
            });
        ui.add_space(24.0);
        ui.label(RichText::new("FOR THE LIVE TOOLS").small().color(theme::MUTED));
        ui.label(
            RichText::new("Channel names, the patch, the mic plan per scene and the DCAs as one exchange file that Show Master and the other Showtime Suite tools can read.")
                .color(theme::MUTED),
        );
        ui.add_space(4.0);
        if ui.button("Export exchange file…").clicked() {
            if let Some(path) = self.pick_exchange() {
                self.export_exchange(&path);
            }
        }
    }

    fn pick_exchange(&self) -> Option<std::path::PathBuf> {
        if !self.dialogs {
            return None;
        }
        let name = self.doc.as_ref().map(|d| d.title()).unwrap_or_else(|| "Untitled".into());
        let mut d = rfd::FileDialog::new()
            .set_title("Export exchange file")
            .add_filter("Showtime exchange", &["json"])
            .set_file_name(format!("{name}.{}", bp_core::exchange::EXTENSION));
        if let Some(dir) = &self.settings.last_dir {
            d = d.set_directory(dir);
        }
        d.save_file()
    }

    /// Write the exchange file. True when written.
    pub fn export_exchange(&mut self, path: &std::path::Path) -> bool {
        let Some(doc) = self.doc.as_ref() else { return false };
        let x = bp_core::exchange::export(&doc.project, &format!("Blueprint {}", env!("CARGO_PKG_VERSION")));
        let r = bp_core::exchange::to_json(&x).and_then(|j| bp_core::project::atomic_write(path, j.as_bytes()).map_err(|e| e.to_string()));
        match r {
            Ok(()) => {
                self.notify(format!("Exported {}", path.display()), false);
                true
            }
            Err(e) => {
                self.notify(format!("Could not export: {e}"), true);
                false
            }
        }
    }

    pub(crate) fn sheet_page(&mut self, ui: &mut egui::Ui, sheet: Sheet) {
        ui.heading(sheet.title());
        ui.label(
            RichText::new(
                "Enter edits · Tab moves on · Cmd/Ctrl+C / V copies and pastes rows from Excel or Numbers · drag the row number to reorder",
            )
            .small()
            .color(theme::MUTED),
        );
        ui.add_space(6.0);
        let hint = format!(
            "No {} yet. Click \"Add {}\" or paste rows from a spreadsheet (click here, then Cmd/Ctrl+V).",
            sheet.title().to_lowercase(),
            sheet.noun()
        );
        self.table(ui, &sheet, Some(sheet.noun()), &hint);
    }

    /// A grid in the table editor, with Add / Delete buttons where rows can
    /// be added. Edits go through [`App::apply`] (with undo).
    pub(crate) fn table(&mut self, ui: &mut egui::Ui, grid: &dyn Grid, add: Option<&str>, empty_hint: &str) {
        let Some(doc) = self.doc.as_ref() else { return };
        let key = grid.key();
        let p = &doc.project;
        let len = grid.len(p);
        let state = self.tables.entry(key.clone()).or_default();
        let mut actions = Vec::new();
        let editable = grid.editable_rows();
        let selected = state.selected.len();
        let cursor = state.cursor.0;
        let mut tool = None;
        ui.horizontal(|ui| {
            if let (Some(noun), true) = (add, editable) {
                if ui
                    .button(format!("Add {noun}"))
                    .on_hover_text("Insert a row below the cursor (Cmd/Ctrl+Enter)")
                    .clicked()
                {
                    let at = if len == 0 { 0 } else { cursor + 1 };
                    actions.push(Action::Insert { at });
                }
                if ui
                    .add_enabled(len > 0 && selected > 0, egui::Button::new("Delete"))
                    .on_hover_text("Delete the selected rows (Cmd/Ctrl+Backspace)")
                    .clicked()
                {
                    actions.push(Action::Delete {
                        rows: state.selected.iter().copied().collect(),
                    });
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(len > 0, egui::Button::new("Export CSV…"))
                    .on_hover_text("The table as a CSV file for Excel, Numbers or another program")
                    .clicked()
                {
                    tool = Some(false);
                }
                if editable
                    && ui
                        .button("Import…")
                        .on_hover_text("Add rows from a CSV, TSV, text or spreadsheet file (XLSX, XLS, ODS); you choose which column goes where")
                        .clicked()
                {
                    tool = Some(true);
                }
            });
        });
        ui.add_space(4.0);
        match tool {
            Some(true) => {
                if let Some(path) = self.pick_import() {
                    self.start_import(grid, &path);
                }
                return;
            }
            Some(false) => {
                if let Some(path) = self.pick_csv(grid) {
                    self.export_csv(grid, &path);
                }
                return;
            }
            None => {}
        }
        if self.import.as_ref().is_some_and(|i| i.key == key) {
            self.import_panel(ui, grid);
        }
        let Some(doc) = self.doc.as_ref() else { return };
        let p = &doc.project;
        let state = self.tables.entry(key.clone()).or_default();
        let columns = grid.columns(p);
        let people: Vec<String> = p.cast.iter().map(|x| x.name.clone()).collect();
        let text = |r: usize, c: usize| grid.text(p, r, c);
        let color = |r: usize| theme::color(&grid.color(p, r));
        let mark = |r: usize, c: usize| grid.mark(p, r, c);
        let view = TableView {
            id: egui::Id::unique(("table", &key)),
            columns: &columns,
            rows: len,
            text: &text,
            color: &color,
            people: &people,
            empty_hint,
            mark: &mark,
            rows_editable: grid.editable_rows(),
        };
        actions.extend(table::show(ui, state, &view));
        for a in actions {
            self.apply(grid, a);
        }
    }

    /// Apply a table action to the project (one undo step each, typing in a
    /// cell merges).
    pub fn apply(&mut self, grid: &dyn Grid, action: Action) {
        let Some(doc) = self.doc.as_mut() else { return };
        let key = grid.key();
        let state = self.tables.entry(key.clone()).or_default();
        let mut message = None;
        match action {
            Action::Set { row, col, value } => {
                let r = doc.edit(&format!("{key}:{row}:{col}"), |p| grid.set(p, row, col, &value));
                if let Err(e) = r {
                    message = Some(e);
                }
            }
            Action::Paste { row, col, text } => {
                doc.seal();
                let (rows, errors) = doc.edit("paste", |p| grid::paste(grid, p, row, col, &text));
                doc.seal();
                state.select(&rows);
                if !errors.is_empty() {
                    let more = if errors.len() > 3 {
                        format!(" (and {} more)", errors.len() - 3)
                    } else {
                        String::new()
                    };
                    message = Some(format!(
                        "Some cells were not pasted: {}{more}",
                        errors[..errors.len().min(3)].join("; ")
                    ));
                }
            }
            Action::Insert { at } => {
                if !grid.editable_rows() {
                    return;
                }
                doc.seal();
                let row = doc.edit("insert", |p| grid.insert(p, at));
                doc.seal();
                state.edit_new(row, 0);
            }
            Action::Delete { rows } => {
                if !grid.editable_rows() {
                    return;
                }
                doc.seal();
                doc.edit("delete", |p| grid.remove(p, &rows));
                doc.seal();
                let first = rows.iter().min().copied().unwrap_or(0);
                let len = grid.len(&doc.project);
                if len > 0 {
                    state.select(&[first.min(len - 1)]);
                } else {
                    state.select(&[]);
                }
            }
            Action::Move { rows, to } => {
                if !grid.editable_rows() {
                    return;
                }
                doc.seal();
                let new = doc.edit("move", |p| grid.move_rows(p, &rows, to));
                doc.seal();
                state.select(&new);
            }
        }
        if let Some(m) = message {
            self.notify(m, true);
        }
    }
}
