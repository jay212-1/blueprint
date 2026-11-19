//! Setlists: the song library and one page per setlist (head fields, times,
//! logo, the entries as a table).

use crate::app::{App, Page};
use crate::theme;
use bp_core::model::Id;
use bp_core::setlist::{parse_start, timing, SetlistGrid, SongGrid};
use bp_core::time::{format_clock, format_duration, parse_duration};
use bp_print::Doc;
use egui::RichText;
use std::path::Path;

/// Logos larger than this are refused (they would make the project file big).
const MAX_LOGO: usize = 2 * 1024 * 1024;

const TABLE_HELP: &str =
    "Enter edits · Tab moves on · Cmd/Ctrl+C / V copies and pastes rows from Excel or Numbers · drag the row number to reorder";

impl App {
    pub(crate) fn songs_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Song library");
        ui.label(
            RichText::new(
                "Every song once. Setlists refer to these songs, so a change here shows in every setlist. Titles typed into a setlist land here too.",
            )
            .color(theme::MUTED),
        );
        ui.label(RichText::new(TABLE_HELP).small().color(theme::MUTED));
        ui.add_space(6.0);
        self.table(
            ui,
            &SongGrid,
            Some("song"),
            "No songs yet. Click \"Add song\", paste a list from a spreadsheet, or type titles straight into a setlist.",
        );
    }

    /// A new setlist, shown at once.
    pub fn new_setlist(&mut self) {
        let Some(doc) = self.doc.as_mut() else { return };
        doc.seal();
        let n = doc.project.setlists.len() + 1;
        let id = doc.edit("setlist:new", |p| p.add_setlist(&format!("Setlist {n}")));
        doc.seal();
        self.page = Page::Setlist(id);
    }

    pub(crate) fn setlist_page(&mut self, ui: &mut egui::Ui, id: Id) {
        let Some(doc) = self.doc.as_mut() else { return };
        let Some(s) = doc.project.setlist(id).cloned() else {
            // Deleted (or undone): back to the library.
            self.page = Page::Songs;
            return;
        };
        let t = timing(&doc.project, &s);
        #[derive(PartialEq)]
        enum Act {
            Delete,
            Duplicate,
            Print,
        }
        let mut act = None;
        ui.horizontal(|ui| {
            ui.heading(if s.name.trim().is_empty() { "Untitled setlist" } else { s.name.as_str() });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Delete setlist")
                    .on_hover_text("Delete this setlist (the songs stay in the library; Cmd/Ctrl+Z brings it back)")
                    .clicked()
                {
                    act = Some(Act::Delete);
                }
                if ui.button("Duplicate").on_hover_text("A copy with the same songs (e.g. for a shorter festival set)").clicked() {
                    act = Some(Act::Duplicate);
                }
                if ui.button(RichText::new("Print…").strong()).clicked() {
                    act = Some(Act::Print);
                }
            });
        });
        match act {
            Some(Act::Delete) => {
                doc.seal();
                doc.edit("setlist:delete", |p| p.setlists.retain(|x| x.id != id));
                doc.seal();
                self.page = Page::Songs;
                self.notify("Setlist deleted. Undo (Cmd/Ctrl+Z) brings it back.".into(), false);
                return;
            }
            Some(Act::Duplicate) => {
                doc.seal();
                let new = doc.edit("setlist:duplicate", |p| p.duplicate_setlist(id));
                doc.seal();
                if let Some(new) = new {
                    self.page = Page::Setlist(new);
                }
                return;
            }
            Some(Act::Print) => {
                self.print.doc = Doc::Setlist(id);
                self.page = Page::Print;
                return;
            }
            None => {}
        }
        ui.label(RichText::new(t.summary()).color(if t.unknown > 0 { theme::WARN } else { theme::MUTED }));
        if t.parts.len() > 1 {
            let parts: Vec<String> = t
                .parts
                .iter()
                .map(|x| format!("{} {} ({} song{})", x.name, format_duration(x.length), x.songs, if x.songs == 1 { "" } else { "s" }))
                .collect();
            ui.label(RichText::new(parts.join("  ·  ")).small().color(theme::MUTED));
        }
        ui.add_space(6.0);

        let mut error = None;
        let mut logo_pick = false;
        let mut logo_clear = false;
        egui::CollapsingHeader::new("Details: band, venue, start time, gap, logo")
            .id_salt(("setlist_head", id))
            .default_open(s.entries.is_empty())
            .show(ui, |ui| {
                egui::Grid::new(("setlist_fields", id)).num_columns(4).spacing([12.0, 8.0]).show(ui, |ui| {
                    text_field(ui, doc, id, "name", "Name", &s.name, |x, v| x.name = v);
                    text_field(ui, doc, id, "band", "Band / artist", &s.band, |x, v| x.band = v);
                    ui.end_row();
                    text_field(ui, doc, id, "venue", "Venue", &s.venue, |x, v| x.venue = v);
                    text_field(ui, doc, id, "date", "Date", &s.date, |x, v| x.date = v);
                    ui.end_row();

                    // Start and gap are parsed when the field is left.
                    ui.label(RichText::new("Start").color(theme::MUTED));
                    let shown = s.start.map(format_clock).unwrap_or_default();
                    if let Some(v) = parsed_field(ui, ("setlist_start", id), &shown, "20:00") {
                        match parse_start(&v) {
                            Ok(start) => {
                                doc.edit(&format!("setlist:{id}:start"), |p| {
                                    if let Some(x) = p.setlist_mut(id) {
                                        x.start = start;
                                    }
                                });
                            }
                            Err(e) => error = Some(e),
                        }
                    }
                    ui.label(RichText::new("Gap between songs").color(theme::MUTED));
                    if let Some(v) = parsed_field(ui, ("setlist_gap", id), &format_duration(s.gap), "0:20") {
                        match parse_duration(&v) {
                            Some(g) => {
                                doc.edit(&format!("setlist:{id}:gap"), |p| {
                                    if let Some(x) = p.setlist_mut(id) {
                                        x.gap = g;
                                    }
                                });
                            }
                            None => error = Some(format!("\"{}\" is not a length (use m:ss)", v.trim())),
                        }
                    }
                    ui.end_row();

                    ui.label(RichText::new("Logo").color(theme::MUTED));
                    ui.horizontal(|ui| {
                        if s.logo.is_empty() {
                            ui.label(RichText::new("none").color(theme::MUTED));
                        } else {
                            let kb = s.logo.len() * 3 / 4 / 1024;
                            ui.label(format!("image, {kb} KB"));
                        }
                        logo_pick = ui.button("Choose…").on_hover_text("PNG, JPEG or SVG: printed in the head").clicked();
                        if !s.logo.is_empty() {
                            logo_clear = ui.button("Remove").clicked();
                        }
                    });
                    ui.end_row();
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Into next: Gap adds the gap above, Attacca and Segue go straight on (printed as an arrow and a tilde).")
                        .small()
                        .color(theme::MUTED),
                );
            });
        if let Some(e) = error {
            self.notify(e, true);
        }
        if logo_clear {
            if let Some(doc) = self.doc.as_mut() {
                doc.seal();
                doc.edit(&format!("setlist:{id}:logo"), |p| {
                    if let Some(x) = p.setlist_mut(id) {
                        x.logo.clear();
                    }
                });
            }
        }
        if logo_pick && self.dialogs {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Choose a logo")
                .add_filter("Image", &["png", "jpg", "jpeg", "svg"])
                .pick_file()
            {
                self.set_logo(id, &path);
            }
        }

        ui.add_space(6.0);
        ui.label(RichText::new(TABLE_HELP).small().color(theme::MUTED));
        ui.add_space(4.0);
        self.table(
            ui,
            &SetlistGrid(id),
            Some("entry"),
            "Empty setlist. Click \"Add entry\" and type a song title (new titles go into the library), or paste a list of titles.",
        );
    }

    /// Read a logo file into a setlist. True when it was taken.
    pub fn set_logo(&mut self, id: Id, path: &Path) -> bool {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.notify(format!("Could not read {}: {e}", path.display()), true);
                return false;
            }
        };
        if bytes.len() > MAX_LOGO {
            self.notify("The logo is larger than 2 MB: please use a smaller image.".into(), true);
            return false;
        }
        if !bp_print::is_image(&bytes) {
            self.notify("This is not a PNG, JPEG or SVG image.".into(), true);
            return false;
        }
        let Some(doc) = self.doc.as_mut() else { return false };
        doc.seal();
        doc.edit(&format!("setlist:{id}:logo"), |p| {
            if let Some(x) = p.setlist_mut(id) {
                x.set_logo(&bytes);
            }
        });
        doc.seal();
        true
    }
}

/// A plain text field of the setlist head (every key stroke is an edit;
/// typing runs are one undo step).
fn text_field(
    ui: &mut egui::Ui,
    doc: &mut bp_core::document::Document,
    id: Id,
    key: &str,
    label: &str,
    value: &str,
    set: fn(&mut bp_core::setlist::Setlist, String),
) {
    ui.label(RichText::new(label).color(theme::MUTED));
    let mut v = value.to_string();
    ui.add(egui::TextEdit::singleline(&mut v).desired_width(220.0).id_salt(("setlist", id, key)));
    if v != value {
        doc.edit(&format!("setlist:{id}:{key}"), |p| {
            if let Some(x) = p.setlist_mut(id) {
                set(x, v);
            }
        });
    }
}

/// A text field that hands back its text once it is left (Enter, Tab or a
/// click elsewhere) and changed; it shows `shown` otherwise.
fn parsed_field(ui: &mut egui::Ui, salt: (&str, Id), shown: &str, hint: &str) -> Option<String> {
    let key = egui::Id::unique(salt).with("buffer");
    let mut buf: String = ui.data(|d| d.get_temp(key)).unwrap_or_else(|| shown.to_string());
    let r = ui.add(egui::TextEdit::singleline(&mut buf).hint_text(hint).desired_width(220.0).id_salt(salt));
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(key, buf));
        None
    } else {
        ui.data_mut(|d| d.remove::<String>(key));
        (r.lost_focus() && buf != shown).then_some(buf)
    }
}
