//! The production page (title, venue … printed on every plan) and the master
//! data lists (cast, roles, scenes, transmitters, capsules, locations).

use crate::app::App;
use crate::table::{self, Action, TableView};
use crate::theme;
use bp_core::model::Meta;
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
    }

    pub(crate) fn sheet_page(&mut self, ui: &mut egui::Ui, sheet: Sheet) {
        let Some(doc) = self.doc.as_ref() else { return };
        let len = sheet.len(&doc.project);
        let state = self.tables.entry(sheet).or_default();
        let selected = state.selected.len();
        let mut actions = Vec::new();
        ui.horizontal(|ui| {
            ui.heading(sheet.title());
            ui.add_space(12.0);
            if ui
                .button(format!("Add {}", sheet.noun()))
                .on_hover_text("Insert a row below the cursor (Cmd/Ctrl+Enter)")
                .clicked()
            {
                let at = if len == 0 { 0 } else { state.cursor.0 + 1 };
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
        });
        ui.label(
            RichText::new(
                "Enter edits · Tab moves on · Cmd/Ctrl+C / V copies and pastes rows from Excel or Numbers · drag the row number to reorder",
            )
            .small()
            .color(theme::MUTED),
        );
        ui.add_space(6.0);

        let p = &doc.project;
        let columns = sheet.columns();
        let people: Vec<String> = p.cast.iter().map(|x| x.name.clone()).collect();
        let text = |r: usize, c: usize| sheet.text(p, r, c);
        let color = |r: usize| theme::color(&sheet.color(p, r));
        let hint = format!(
            "No {} yet. Click \"Add {}\" or paste rows from a spreadsheet (click here, then Cmd/Ctrl+V).",
            sheet.title().to_lowercase(),
            sheet.noun()
        );
        let view = TableView {
            id: egui::Id::unique(("sheet", sheet)),
            columns: &columns,
            rows: len,
            text: &text,
            color: &color,
            people: &people,
            empty_hint: &hint,
        };
        actions.extend(table::show(ui, state, &view));
        for a in actions {
            self.apply(sheet, a);
        }
    }

    /// Apply a table action to the project (one undo step each, typing in a
    /// cell merges).
    pub fn apply(&mut self, sheet: Sheet, action: Action) {
        let Some(doc) = self.doc.as_mut() else { return };
        let state = self.tables.entry(sheet).or_default();
        let mut message = None;
        match action {
            Action::Set { row, col, value } => {
                let id = sheet.id(&doc.project, row).unwrap_or(0);
                let r = doc.edit(&format!("{sheet:?}:{id}:{col}"), |p| sheet.set(p, row, col, &value));
                if let Err(e) = r {
                    message = Some(e);
                }
            }
            Action::Paste { row, col, text } => {
                doc.seal();
                let (rows, errors) = doc.edit("paste", |p| sheet.paste(p, row, col, &text));
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
                doc.seal();
                let row = doc.edit("insert", |p| sheet.insert(p, at));
                doc.seal();
                state.edit_new(row, 0);
            }
            Action::Delete { rows } => {
                doc.seal();
                doc.edit("delete", |p| sheet.remove(p, &rows));
                doc.seal();
                let first = rows.iter().min().copied().unwrap_or(0);
                let len = sheet.len(&doc.project);
                if len > 0 {
                    state.select(&[first.min(len - 1)]);
                } else {
                    state.select(&[]);
                }
            }
            Action::Move { rows, to } => {
                doc.seal();
                let new = doc.edit("move", |p| sheet.move_rows(p, &rows, to));
                doc.seal();
                state.select(&new);
            }
        }
        if let Some(m) = message {
            self.notify(m, true);
        }
    }
}
