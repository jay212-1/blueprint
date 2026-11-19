//! The other plans: one page per plan (head, the plan's tables, and the
//! stage editor for stage plots).

use crate::app::{App, Page};
use crate::app_setlist::parsed_field;
use crate::theme;
use bp_core::model::Id;
use bp_core::plan_grid::{Part, PlanGrid};
use bp_core::plans::{add_cast, refresh_packlist, run_times, PlanKind, PlanType};
use bp_core::time::{format_clock, format_number, parse_number};
use bp_print::Doc;
use egui::RichText;

const TABLE_HELP: &str =
    "Enter edits · Tab moves on · Cmd/Ctrl+C / V copies and pastes rows from Excel or Numbers · drag the row number to reorder";

impl App {
    /// A new plan of this kind, shown at once.
    pub fn new_plan(&mut self, t: PlanType) -> Option<Id> {
        let doc = self.doc.as_mut()?;
        doc.seal();
        let id = doc.edit("plan:new", |p| p.add_plan(t));
        doc.seal();
        self.page = Page::Plan(id);
        Some(id)
    }

    pub(crate) fn plan_page(&mut self, ui: &mut egui::Ui, id: Id) {
        let Some(plan) = self.doc.as_ref().and_then(|d| d.project.plan(id)).cloned() else {
            self.page = Page::Production;
            return;
        };
        let t = plan.kind.plan_type();
        #[derive(PartialEq)]
        enum Act {
            Delete,
            Duplicate,
            Print,
        }
        let mut act = None;
        ui.horizontal(|ui| {
            ui.heading(if plan.name.trim().is_empty() { t.label() } else { plan.name.as_str() });
            ui.label(RichText::new(t.label()).small().color(theme::MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Delete plan").on_hover_text("Cmd/Ctrl+Z brings it back").clicked() {
                    act = Some(Act::Delete);
                }
                if ui.button("Duplicate").clicked() {
                    act = Some(Act::Duplicate);
                }
                if ui.button(RichText::new("Print…").strong()).clicked() {
                    act = Some(Act::Print);
                }
            });
        });
        ui.label(RichText::new(t.hint()).small().color(theme::MUTED));
        let Some(doc) = self.doc.as_mut() else { return };
        match act {
            Some(Act::Delete) => {
                doc.seal();
                doc.edit("plan:delete", |p| p.plans.retain(|x| x.id != id));
                doc.seal();
                self.page = Page::Production;
                self.notify("Plan deleted. Undo (Cmd/Ctrl+Z) brings it back.".into(), false);
                return;
            }
            Some(Act::Duplicate) => {
                doc.seal();
                let new = doc.edit("plan:duplicate", |p| p.duplicate_plan(id));
                doc.seal();
                if let Some(new) = new {
                    self.page = Page::Plan(new);
                }
                return;
            }
            Some(Act::Print) => {
                self.print.doc = Doc::Plan(id);
                self.page = Page::Print;
                return;
            }
            None => {}
        }

        // Name and notes.
        egui::CollapsingHeader::new("Name and notes")
            .id_salt(("plan_head", id))
            .default_open(false)
            .show(ui, |ui| {
                let mut name = plan.name.clone();
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Name").color(theme::MUTED));
                    ui.add(egui::TextEdit::singleline(&mut name).desired_width(300.0).id_salt(("plan_name", id)));
                });
                if name != plan.name {
                    doc.edit(&format!("plan:{id}:name"), |p| {
                        if let Some(x) = p.plan_mut(id) {
                            x.name = name;
                        }
                    });
                }
                let mut notes = plan.notes.clone();
                ui.add(egui::TextEdit::multiline(&mut notes).desired_rows(2).desired_width(600.0).hint_text("Notes (printed under the plan)").id_salt(("plan_notes", id)));
                if notes != plan.notes {
                    doc.edit(&format!("plan:{id}:notes"), |p| {
                        if let Some(x) = p.plan_mut(id) {
                            x.notes = notes;
                        }
                    });
                }
            });

        // What each kind has on top of its tables.
        match &plan.kind {
            PlanKind::RunOfShow(r) => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Date").color(theme::MUTED));
                    let mut date = r.date.clone();
                    ui.add(egui::TextEdit::singleline(&mut date).desired_width(200.0).id_salt(("run_date", id)));
                    if date != r.date {
                        doc.edit(&format!("plan:{id}:date"), |p| {
                            if let Some(PlanKind::RunOfShow(r)) = p.plan_mut(id).map(|x| &mut x.kind) {
                                r.date = date;
                            }
                        });
                    }
                    let times = run_times(r);
                    let first = times.iter().find_map(|t| t.0);
                    let last = times.iter().rev().find_map(|t| t.1);
                    if let (Some(a), Some(b)) = (first, last) {
                        ui.label(RichText::new(format!("{} to {} · {} rows", format_clock(a), format_clock(b), r.rows.len())).color(theme::MUTED));
                    }
                });
                ui.label(RichText::new("A row without a start time follows the row above.").small().color(theme::MUTED));
            }
            PlanKind::Contacts(_) => {
                if ui.button("Add the cast").on_hover_text("Everyone from the cast who is not on the list yet").clicked() {
                    doc.seal();
                    let n = doc.edit(&format!("plan:{id}:cast"), |p| add_cast(p, id));
                    doc.seal();
                    self.notify(format!("{n} added from the cast."), false);
                }
            }
            PlanKind::Packlist(k) => {
                let packed = k.rows.iter().filter(|r| r.packed).count();
                let mut count = false;
                ui.horizontal(|ui| {
                    count = ui
                        .button("Count from patch and mic plan")
                        .on_hover_text("Adds and updates the counted rows (cables, DI boxes, mics, stands, transmitters, capsules, batteries); your own rows and the packed marks stay")
                        .clicked();
                    ui.label(RichText::new(format!("{packed} of {} packed", k.rows.len())).color(if packed == k.rows.len() && packed > 0 { theme::OK } else { theme::MUTED }));
                });
                if count {
                    doc.seal();
                    let (a, c, r) = doc.edit(&format!("plan:{id}:count"), |p| refresh_packlist(p, id));
                    doc.seal();
                    self.notify(format!("Counted: {a} new, {c} changed, {r} removed."), false);
                }
            }
            PlanKind::StagePlot(s) => {
                let s = s.clone();
                self.stage_settings(ui, id, &s);
                self.stage_editor(ui, id, &s);
            }
            _ => {}
        }

        // The plan's tables (two of them: tabs).
        let parts = Part::of(&plan.kind);
        let mut part = self.plan_part.get(&id).copied().filter(|p| parts.contains(p)).unwrap_or(parts[0]);
        if parts.len() > 1 {
            ui.horizontal(|ui| {
                for p in parts {
                    ui.selectable_value(&mut part, *p, p.title());
                }
            });
            self.plan_part.insert(id, part);
        }
        ui.label(RichText::new(TABLE_HELP).small().color(theme::MUTED));
        ui.add_space(4.0);
        let hint = match part {
            Part::IcPeople => "No one yet. First add the channels (tab \"Channels\"), then the people: type Talk or Listen under each channel.",
            Part::StageItems => "Nothing on stage yet. Add symbols with the buttons above, or rows here.",
            Part::Rows => "No rows yet. Set up the columns (tab \"Columns\"), then add rows or paste them.",
            _ => "Empty. Click \"Add\" or paste rows from a spreadsheet (click here, then Cmd/Ctrl+V).",
        };
        self.table(ui, &PlanGrid { plan: id, part }, Some(part.noun()), hint);
    }

    fn stage_settings(&mut self, ui: &mut egui::Ui, id: Id, s: &bp_core::stage::StagePlot) {
        let Some(doc) = self.doc.as_mut() else { return };
        let mut error = None;
        ui.horizontal_wrapped(|ui| {
            let mut num = |ui: &mut egui::Ui, key: &str, label: &str, v: f64, set: fn(&mut bp_core::stage::StagePlot, f64)| {
                ui.label(RichText::new(label).color(theme::MUTED));
                let shown = format_number(v);
                ui.scope(|ui| {
                    ui.set_max_width(60.0);
                    if let Some(t) = parsed_field(ui, (key, id), &shown, "") {
                        match parse_number(&t).filter(|x| *x > 0.0 && *x < 500.0) {
                            Some(x) => {
                                doc.edit(&format!("plan:{id}:{key}"), |p| {
                                    if let Some(PlanKind::StagePlot(s)) = p.plan_mut(id).map(|x| &mut x.kind) {
                                        set(s, x);
                                    }
                                });
                            }
                            None => error = Some(format!("\"{}\" is not a size in metres", t.trim())),
                        }
                    }
                });
            };
            num(ui, "stage_w", "Stage width (m)", s.width, |s, v| s.width = v);
            num(ui, "stage_d", "Depth (m)", s.depth, |s, v| s.depth = v);
            num(ui, "stage_g", "Grid (m)", s.grid, |s, v| s.grid = v);
            let mut g = s.show_grid;
            if ui.checkbox(&mut g, "Show grid").changed() {
                doc.edit(&format!("plan:{id}:grid_on"), |p| {
                    if let Some(PlanKind::StagePlot(s)) = p.plan_mut(id).map(|x| &mut x.kind) {
                        s.show_grid = g;
                    }
                });
            }
            ui.label(RichText::new("Front").color(theme::MUTED));
            let mut f = s.front.clone();
            ui.add(egui::TextEdit::singleline(&mut f).desired_width(120.0).id_salt(("stage_front", id)));
            if f != s.front {
                doc.edit(&format!("plan:{id}:front"), |p| {
                    if let Some(PlanKind::StagePlot(s)) = p.plan_mut(id).map(|x| &mut x.kind) {
                        s.front = f;
                    }
                });
            }
        });
        if let Some(e) = error {
            self.notify(e, true);
        }
    }
}
