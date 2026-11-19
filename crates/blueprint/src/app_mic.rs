//! The mic plan pages: matrix, transmitters per role, performers, swaps,
//! casts of the day, DCAs, batteries, frequencies, and the list of checks.

use crate::app::{App, Page};
use crate::theme;
use bp_core::mic::{replace_tx, tx_label, MicView, Problem};
use bp_core::mic_grid::{MicGrid, MicSheet};
use bp_core::model::TxStatus;
use egui::RichText;
use std::sync::Arc;

impl App {
    /// The plan worked out for the project as it is now (cached until it changes).
    pub fn mic_view(&mut self) -> Arc<MicView> {
        let Some(doc) = self.doc.as_ref() else {
            return Arc::new(MicView::default());
        };
        match &self.mic_cache {
            Some((g, v)) if *g == doc.generation => v.clone(),
            _ => {
                let v = Arc::new(MicView::build(&doc.project));
                self.mic_cache = Some((doc.generation, v.clone()));
                v
            }
        }
    }

    pub(crate) fn mic_page(&mut self, ui: &mut egui::Ui, sheet: MicSheet) {
        let view = self.mic_view();
        ui.heading(sheet.title());
        ui.label(RichText::new(sheet.help()).small().color(theme::MUTED));
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if matches!(
                sheet,
                MicSheet::Plot | MicSheet::Tracks | MicSheet::Performers | MicSheet::Swaps | MicSheet::Variants
            ) {
                self.cast_picker(ui);
            }
            match sheet {
                MicSheet::Swaps => self.swap_settings(ui),
                MicSheet::Dca => self.dca_settings(ui),
                MicSheet::Frequencies => self.frequency_settings(ui),
                MicSheet::Tracks => self.spare_panel(ui),
                _ => {}
            }
        });
        if sheet == MicSheet::Variants {
            self.variant_list(ui);
        }
        if sheet == MicSheet::Plot {
            let n = view.problems.len();
            if n > 0 {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{n} problem{} in the plan", if n == 1 { "" } else { "s" })).color(theme::WARN));
                    if ui.small_button("Show checks").clicked() {
                        self.page = Page::Checks;
                    }
                    ui.label(RichText::new("Red frames mark them; rest the pointer on a cell for details.").small().color(theme::MUTED));
                });
            }
        }
        ui.add_space(4.0);
        let grid = MicGrid { sheet, view: &view };
        let hint = match sheet {
            MicSheet::Plot | MicSheet::Dca => "Add scenes first (Master data › Scenes).",
            MicSheet::Tracks | MicSheet::Variants => "Add roles first (Master data › Roles).",
            MicSheet::Performers => "Add the cast first (Master data › Cast).",
            MicSheet::Swaps => "No swaps: every transmitter stays with one person.",
            MicSheet::Battery | MicSheet::Frequencies => "Add transmitters first (Master data › Transmitters).",
        };
        self.table(ui, &grid, None, hint);
    }

    /// "Shown for: First cast / Cast B …".
    fn cast_picker(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = self.doc.as_mut() else { return };
        let plan = &doc.project.mic;
        if plan.variants.is_empty() && !matches!(self.page, Page::Mic(MicSheet::Variants)) {
            return;
        }
        let current = plan.variant().map(|v| v.name.clone()).unwrap_or_else(|| "First cast".into());
        let mut pick = plan.variant;
        ui.label(RichText::new("Cast").color(theme::MUTED));
        egui::ComboBox::from_id_salt("cast_of_the_day")
            .selected_text(current)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut pick, None, "First cast");
                for v in &plan.variants {
                    ui.selectable_value(&mut pick, Some(v.id), &v.name);
                }
            });
        if pick != plan.variant {
            doc.seal();
            doc.edit("mic:variant", |p| p.mic.variant = pick);
            doc.seal();
        }
        ui.add_space(12.0);
    }

    fn swap_settings(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = self.doc.as_mut() else { return };
        let mut v = doc.project.mic.min_swap;
        ui.label(RichText::new("A swap needs at least").color(theme::MUTED));
        if ui
            .add(egui::DragValue::new(&mut v).range(0.0..=900.0).speed(1.0).suffix(" s"))
            .changed()
        {
            doc.edit("mic:min_swap", |p| p.mic.min_swap = v);
        }
    }

    fn dca_settings(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = self.doc.as_mut() else { return };
        let mut v = doc.project.mic.dca_count;
        ui.label(RichText::new("DCAs on the desk").color(theme::MUTED));
        if ui.add(egui::DragValue::new(&mut v).range(1..=96)).changed() {
            doc.edit("mic:dca_count", |p| p.mic.dca_count = v);
        }
    }

    fn frequency_settings(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = self.doc.as_mut() else { return };
        let mut on = doc.project.mic.im3;
        if ui
            .checkbox(&mut on, "Check intermodulation (3rd order)")
            .on_hover_text("Warns when 2 × f1 − f2 of two transmitters lands near a third one.")
            .changed()
        {
            doc.seal();
            doc.edit("mic:im3", |p| p.mic.im3 = on);
            doc.seal();
        }
        if on {
            let mut g = doc.project.mic.im3_guard_khz;
            ui.label(RichText::new("closer than").color(theme::MUTED));
            if ui
                .add(egui::DragValue::new(&mut g).range(1.0..=2000.0).speed(5.0).suffix(" kHz"))
                .changed()
            {
                doc.edit("mic:im3_guard", |p| p.mic.im3_guard_khz = g);
            }
        }
    }

    /// "Transmitter 12 is broken → spare S2 takes over".
    fn spare_panel(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = self.doc.as_ref() else { return };
        let p = &doc.project;
        let spares: Vec<_> = p.transmitters.iter().filter(|t| t.status == TxStatus::Spare).map(|t| t.id).collect();
        if spares.is_empty() || p.transmitters.is_empty() {
            return;
        }
        let (mut broken, mut spare) = self.spare_pick;
        if !p.transmitters.iter().any(|t| Some(t.id) == broken && t.status == TxStatus::Active) {
            broken = p.transmitters.iter().find(|t| t.status == TxStatus::Active).map(|t| t.id);
        }
        if spare.is_none_or(|s| !spares.contains(&s)) {
            spare = spares.first().copied();
        }
        let label = |id: Option<bp_core::model::Id>| id.map(|i| format!("TX {}", tx_label(p, i))).unwrap_or_default();
        ui.label(RichText::new("Transmitter out:").color(theme::MUTED));
        egui::ComboBox::from_id_salt("broken_tx").selected_text(label(broken)).show_ui(ui, |ui| {
            for t in p.transmitters.iter().filter(|t| t.status == TxStatus::Active) {
                ui.selectable_value(&mut broken, Some(t.id), label(Some(t.id)));
            }
        });
        ui.label(RichText::new("spare taking over:").color(theme::MUTED));
        egui::ComboBox::from_id_salt("spare_tx").selected_text(label(spare)).show_ui(ui, |ui| {
            for s in &spares {
                ui.selectable_value(&mut spare, Some(*s), label(Some(*s)));
            }
        });
        let go = ui
            .add_enabled(broken.is_some() && spare.is_some(), egui::Button::new("Swap in the spare"))
            .on_hover_text("The spare takes over everywhere in the plan; the transmitter is marked defect.");
        self.spare_pick = (broken, spare);
        if go.clicked() {
            if let (Some(b), Some(s), Some(doc)) = (broken, spare, self.doc.as_mut()) {
                let names = (tx_label(&doc.project, b), tx_label(&doc.project, s));
                doc.seal();
                let r = doc.edit("mic:spare", |p| replace_tx(p, b, s));
                doc.seal();
                match r {
                    Ok(()) => self.notify(format!("Spare {} takes over from transmitter {} (marked defect).", names.1, names.0), false),
                    Err(e) => self.notify(e, true),
                }
            }
        }
    }

    /// Names of the casts of the day, with add and delete.
    fn variant_list(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = self.doc.as_mut() else { return };
        ui.add_space(4.0);
        let mut remove = None;
        let n = doc.project.mic.variants.len();
        for i in 0..n {
            ui.horizontal(|ui| {
                let mut name = doc.project.mic.variants[i].name.clone();
                let id = doc.project.mic.variants[i].id;
                if ui
                    .add(egui::TextEdit::singleline(&mut name).desired_width(200.0).hint_text("Name of the cast"))
                    .changed()
                {
                    doc.edit(&format!("mic:variant_name:{id}"), |p| p.mic.variants[i].name = name);
                }
                let mut notes = doc.project.mic.variants[i].notes.clone();
                if ui
                    .add(egui::TextEdit::singleline(&mut notes).desired_width(320.0).hint_text("Notes (when it plays)"))
                    .changed()
                {
                    doc.edit(&format!("mic:variant_notes:{id}"), |p| p.mic.variants[i].notes = notes);
                }
                if ui.small_button("Delete").clicked() {
                    remove = Some(i);
                }
            });
        }
        if ui.button("Add a cast").clicked() {
            doc.seal();
            doc.edit("mic:variant_add", |p| {
                let id = p.alloc_id();
                let n = p.mic.variants.len() + 2;
                p.mic.variants.push(bp_core::mic::CastVariant {
                    id,
                    name: format!("Cast {}", (b'A' + n as u8 - 1) as char),
                    ..Default::default()
                });
            });
            doc.seal();
        }
        if let Some(i) = remove {
            doc.seal();
            doc.edit("mic:variant_remove", |p| {
                let v = p.mic.variants.remove(i);
                if p.mic.variant == Some(v.id) {
                    p.mic.variant = None;
                }
            });
            doc.seal();
        }
    }

    pub(crate) fn checks_page(&mut self, ui: &mut egui::Ui) {
        let view = self.mic_view();
        let Some(doc) = self.doc.as_ref() else { return };
        let p = &doc.project;
        ui.heading("Checks");
        ui.label(
            RichText::new("Problems in the mic plan, worked out from the plan as it is now.")
                .small()
                .color(theme::MUTED),
        );
        ui.add_space(8.0);
        if view.problems.is_empty() {
            ui.label(RichText::new("No problems found.").color(theme::OK));
            return;
        }
        let mut go: Option<(Page, Option<usize>, Option<usize>)> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, x) in view.problems.iter().enumerate() {
                ui.horizontal(|ui| {
                    let (icon_color, kind) = match x {
                        Problem::DoubleBooked { .. } | Problem::NoMic { .. } | Problem::PersonTwice { .. } => (theme::ERROR, "Plan"),
                        Problem::SwapTooShort { .. } => (theme::WARN, "Swap"),
                        Problem::NotActive { .. } => (theme::WARN, "Transmitter"),
                        Problem::DuplicateFrequency { .. } | Problem::Intermod { .. } => (theme::WARN, "Frequency"),
                        Problem::NoPerformer { .. } => (theme::WARN, "Cast"),
                    };
                    let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 4.5, icon_color);
                    ui.label(RichText::new(kind).small().color(theme::MUTED));
                    ui.label(x.message(p));
                    let (scene, tx) = x.place();
                    let target = match x {
                        Problem::DuplicateFrequency { .. } | Problem::Intermod { .. } => Some(Page::Mic(MicSheet::Frequencies)),
                        Problem::NoPerformer { .. } => Some(Page::Mic(MicSheet::Variants)),
                        Problem::NotActive { .. } => Some(Page::Mic(MicSheet::Tracks)),
                        _ => scene.map(|_| Page::Mic(MicSheet::Plot)),
                    };
                    if let Some(t) = target {
                        if ui.small_button("Show").on_hover_text("Go to the place in the plan").clicked() {
                            let col = tx.and_then(|t| view.column_of(t)).map(|c| c + 2);
                            go = Some((t, scene, col));
                        }
                    }
                    let _ = i;
                });
            }
        });
        if let Some((page, row, col)) = go {
            self.page = page;
            if let (Page::Mic(MicSheet::Plot), Some(row)) = (page, row) {
                let st = self.tables.entry(format!("mic:{:?}", MicSheet::Plot)).or_default();
                st.select(&[row]);
                st.cursor = (row, col.unwrap_or(1));
            }
        }
    }
}
