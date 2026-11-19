//! Print: choose a document, paper, orientation and theme, see the pages,
//! issue a revision, export the PDF.

use crate::app::App;
use crate::theme;
use bp_core::mic::MicView;
use bp_core::model::Id;
use bp_core::time::today;
use bp_print::{Doc, Options, Paper};
use egui::{RichText, TextureHandle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Pages rendered for the preview at most.
const MAX_PREVIEW_PAGES: usize = 40;
/// Wait this long after the last change before laying out again.
const SETTLE: Duration = Duration::from_millis(350);

pub struct PrintState {
    pub doc: Doc,
    pub opts: HashMap<Doc, Options>,
    preview: Option<Preview>,
    /// What the preview should show, and since when.
    wanted: Option<(u64, Doc, Options, Instant)>,
}

impl Default for PrintState {
    fn default() -> Self {
        Self {
            doc: Doc::MicPlot,
            opts: HashMap::new(),
            preview: None,
            wanted: None,
        }
    }
}

impl PrintState {
    pub fn options(&self, doc: Doc) -> Options {
        self.opts.get(&doc).copied().unwrap_or_else(|| Options::for_doc(doc))
    }
}

struct Preview {
    generation: u64,
    doc: Doc,
    opts: Options,
    pages: Vec<TextureHandle>,
    total: usize,
    warnings: Vec<String>,
    fit: Option<f64>,
    error: Option<String>,
    took: Duration,
}

impl App {
    pub(crate) fn print_page(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = self.doc.as_ref() else { return };
        let generation = doc.generation;
        // A setlist that is gone (deleted, undone) falls back to the mic plot.
        if let Doc::Setlist(id) = self.print.doc {
            if doc.project.setlist(id).is_none() {
                self.print.doc = Doc::MicPlot;
            }
        }
        let lists: Vec<(Id, String)> = doc
            .project
            .setlists
            .iter()
            .map(|s| (s.id, if s.name.trim().is_empty() { "Untitled setlist".into() } else { s.name.clone() }))
            .collect();
        let current = self.print.doc;
        let mut opts = self.print.options(current);
        let mut pick = current;

        egui::Panel::left("print_options")
            .resizable(false)
            .exact_size(270.0)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin { left: 0, right: 14, top: 0, bottom: 0 }))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.heading("Print");
                    ui.add_space(6.0);
                    ui.label(RichText::new("MIC PLAN").small().color(theme::MUTED));
                    for d in Doc::MIC {
                        if ui
                            .add(egui::Button::selectable(pick == d, d.title()).min_size(egui::vec2(ui.available_width(), 26.0)))
                            .clicked()
                        {
                            pick = d;
                        }
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new("PATCH").small().color(theme::MUTED));
                    for d in Doc::PATCH {
                        if ui
                            .add(egui::Button::selectable(pick == d, d.title()).min_size(egui::vec2(ui.available_width(), 26.0)))
                            .clicked()
                        {
                            pick = d;
                        }
                    }
                    if !lists.is_empty() {
                        ui.add_space(6.0);
                        ui.label(RichText::new("SETLISTS").small().color(theme::MUTED));
                    }
                    for (id, name) in &lists {
                        let d = Doc::Setlist(*id);
                        if ui
                            .add(egui::Button::selectable(pick == d, name.as_str()).min_size(egui::vec2(ui.available_width(), 26.0)))
                            .clicked()
                        {
                            pick = d;
                        }
                    }
                    ui.add_space(10.0);
                    ui.label(RichText::new("PAGE").small().color(theme::MUTED));
                    ui.horizontal(|ui| {
                        for p in Paper::ALL {
                            ui.selectable_value(&mut opts.paper, p, p.label());
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut opts.landscape, false, "Portrait");
                        ui.selectable_value(&mut opts.landscape, true, "Landscape");
                    });
                    ui.add_space(10.0);
                    ui.label(RichText::new("THEME").small().color(theme::MUTED));
                    for t in pick.themes() {
                        ui.radio_value(&mut opts.theme, *t, t.label_for(pick)).on_hover_text(t.hint(pick));
                    }
                    if let Doc::Setlist(_) = pick {
                        ui.label(
                            RichText::new("The type is as large as fits on one page.").small().color(theme::MUTED),
                        );
                    }
                    if pick == Doc::MicPlot {
                        ui.add_space(6.0);
                        ui.checkbox(&mut opts.mark_changes, "Mark changes since the revision before");
                    }
                    if pick.has_revision() {
                        ui.add_space(12.0);
                        ui.label(RichText::new("REVISION").small().color(theme::MUTED));
                        self.revision_box(ui);
                    }
                    ui.add_space(12.0);
                    if ui
                        .add_sized([ui.available_width(), 34.0], egui::Button::new(RichText::new("Export PDF…").strong()))
                        .clicked()
                    {
                        if let Some(path) = self.pick_pdf(pick) {
                            self.export_pdf(pick, &path);
                        }
                    }
                });
            });

        if pick != current {
            self.print.doc = pick;
            opts = self.print.options(pick);
        }
        self.print.opts.insert(pick, opts);

        // Lay out again once things have settled.
        let key = (generation, pick, opts);
        let shown = self
            .print
            .preview
            .as_ref()
            .is_some_and(|p| (p.generation, p.doc, p.opts) == key);
        if !shown {
            let since = match self.print.wanted {
                Some((g, d, o, t)) if (g, d, o) == key => t,
                _ => {
                    let now = Instant::now();
                    self.print.wanted = Some((generation, pick, opts, now));
                    now
                }
            };
            // The first preview of a document comes at once.
            let first = self.print.preview.as_ref().is_none_or(|p| p.doc != pick);
            if first || since.elapsed() >= SETTLE {
                self.render_preview(ui.ctx(), pick, opts, generation);
            } else {
                ui.ctx().request_repaint_after(SETTLE);
            }
        }

        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| self.preview_pages(ui));
    }

    fn revision_box(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = self.doc.as_mut() else { return };
        let r = &doc.project.mic.revision;
        if r.number == 0 {
            ui.label("Draft (no revision issued yet)");
        } else {
            ui.label(format!("Rev {} · {}", r.number, r.date));
        }
        let next = r.number + 1;
        if ui
            .button(format!("Issue Rev {next}"))
            .on_hover_text("Prints from now on carry the new revision and today's date. Changes after it are marked against this one.")
            .clicked()
        {
            doc.seal();
            doc.edit("mic:revision", |p| {
                let snap = MicView::build(p).snapshot(p);
                p.mic.revision.issue(snap, today());
            });
            doc.seal();
            self.notify(format!("Revision {next} issued."), false);
        }
    }

    fn render_preview(&mut self, ctx: &egui::Context, doc: Doc, opts: Options, generation: u64) {
        let Some(d) = self.doc.as_ref() else { return };
        let started = Instant::now();
        let mut pages = Vec::new();
        let mut warnings = Vec::new();
        let mut fit = None;
        let (total, error) = match bp_print::print(&d.project, doc, &opts) {
            Ok(printed) => {
                for i in 0..printed.pages().min(MAX_PREVIEW_PAGES) {
                    if let Some((w, h, rgba)) = printed.render(i, 1.5) {
                        let img = egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], &rgba);
                        pages.push(ctx.load_texture(format!("print_page_{i}"), img, egui::TextureOptions::LINEAR));
                    }
                }
                warnings = printed.warnings.clone();
                fit = printed.fit;
                (printed.pages(), None)
            }
            Err(e) => (0, Some(e)),
        };
        self.print.preview = Some(Preview { generation, doc, opts, pages, total, warnings, fit, error, took: started.elapsed() });
        self.print.wanted = None;
    }

    fn preview_pages(&mut self, ui: &mut egui::Ui) {
        let Some(pv) = &self.print.preview else {
            ui.label(RichText::new("Laying out…").color(theme::MUTED));
            return;
        };
        if let Some(e) = &pv.error {
            ui.label(RichText::new(e).color(theme::ERROR));
            return;
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} page{} · laid out in {} ms",
                    pv.total,
                    if pv.total == 1 { "" } else { "s" },
                    pv.took.as_millis()
                ))
                .small()
                .color(theme::MUTED),
            );
            if pv.total > pv.pages.len() {
                ui.label(RichText::new(format!("(preview shows the first {})", pv.pages.len())).small().color(theme::MUTED));
            }
            if let Some(f) = pv.fit {
                ui.label(RichText::new(format!("· type fitted at {f:.1} pt")).small().color(theme::MUTED));
            }
        });
        for w in &pv.warnings {
            ui.label(RichText::new(w).color(theme::WARN));
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let w = (ui.available_width() - 8.0).clamp(200.0, 1100.0);
            for t in &pv.pages {
                let size = t.size_vec2();
                let h = w * size.y / size.x;
                ui.add_space(6.0);
                let img = egui::Image::new((t.id(), egui::vec2(w, h)));
                egui::Frame::new()
                    .stroke(egui::Stroke::new(1.0, theme::LINE))
                    .show(ui, |ui| ui.add(img));
            }
            ui.add_space(12.0);
        });
    }

    fn pick_pdf(&self, doc: Doc) -> Option<PathBuf> {
        if !self.dialogs {
            return None;
        }
        let name = self.doc.as_ref().map(|d| d.title()).unwrap_or_else(|| "Untitled".into());
        let mut d = rfd::FileDialog::new()
            .set_title("Export PDF")
            .add_filter("PDF", &["pdf"])
            .set_file_name(format!("{name} - {}.pdf", self.doc_name(doc)));
        if let Some(dir) = &self.settings.last_dir {
            d = d.set_directory(dir);
        }
        d.save_file().map(|p| if p.extension().is_none() { p.with_extension("pdf") } else { p })
    }

    /// The document's name for file names ("Mic plot", the setlist's name).
    fn doc_name(&self, doc: Doc) -> String {
        match doc {
            Doc::Setlist(id) => self
                .doc
                .as_ref()
                .and_then(|d| d.project.setlist(id))
                .map(|s| s.name.trim().to_string())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| "Setlist".into()),
            _ => doc.title().into(),
        }
    }

    /// Lay out `doc` and write the PDF. True when written.
    pub fn export_pdf(&mut self, doc: Doc, path: &Path) -> bool {
        let Some(d) = self.doc.as_ref() else { return false };
        let opts = self.print.options(doc);
        let r = bp_print::print(&d.project, doc, &opts)
            .and_then(|p| p.pdf())
            .and_then(|pdf| bp_core::project::atomic_write(path, &pdf).map_err(|e| e.to_string()));
        match r {
            Ok(()) => {
                self.notify(format!("Exported {}", path.display()), false);
                true
            }
            Err(e) => {
                self.notify(format!("Could not export the PDF: {e}"), true);
                false
            }
        }
    }
}
