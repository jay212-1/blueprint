//! The Blueprint window: start page, navigation, the open project with undo,
//! saving, autosave and recovery. The pages live in `app_*.rs`.

use crate::settings::{self, Settings};
use crate::table::TableState;
use crate::theme;
use bp_core::document::Document;
use bp_core::model::Project;
use bp_core::project;
use bp_core::mic::MicView;
use bp_core::mic_grid::MicSheet;
use bp_core::model::Id;
use bp_core::sheet::Sheet;
use egui::{Color32, Key, KeyboardShortcut, Modifiers, RichText, ViewportCommand};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Unsaved changes are written to the autosave file this often.
const AUTOSAVE_EVERY: Duration = Duration::from_secs(20);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Production,
    Sheet(Sheet),
    Mic(MicSheet),
    Checks,
    Print,
    Songs,
    Setlist(Id),
}

/// What to do once unsaved changes are dealt with.
#[derive(Clone, Debug, PartialEq)]
enum After {
    New,
    Demo,
    DemoBand,
    Open(Option<PathBuf>),
    Close,
    Quit,
}

enum Modal {
    /// An autosave newer than the file (or of an untitled project) exists.
    Recover {
        path: Option<PathBuf>,
        project: Box<Project>,
    },
    Unsaved(After),
    Message(String),
}

pub struct App {
    pub doc: Option<Document>,
    pub page: Page,
    pub tables: HashMap<String, TableState>,
    pub settings: Settings,
    /// File dialogs (off in tests).
    pub dialogs: bool,
    modal: Option<Modal>,
    toast: Option<(String, bool, Instant)>,
    last_autosave: Instant,
    allow_close: bool,
    window_title: String,
    /// An untitled project that was not saved last time.
    pub untitled_recovery: Option<Project>,
    /// The mic plan worked out for a document generation.
    pub(crate) mic_cache: Option<(u64, std::sync::Arc<MicView>)>,
    /// Transmitter out and spare chosen in the spare panel.
    pub(crate) spare_pick: (Option<Id>, Option<Id>),
    pub print: crate::app_print::PrintState,
}

impl App {
    pub fn new(ctx: &egui::Context, open: Option<PathBuf>) -> Self {
        theme::apply(ctx);
        let untitled_recovery = settings::untitled_autosave().and_then(|p| project::load(&p).ok());
        let mut app = App {
            doc: None,
            page: Page::Production,
            tables: HashMap::new(),
            settings: Settings::load(),
            dialogs: true,
            modal: None,
            toast: None,
            last_autosave: Instant::now(),
            allow_close: false,
            window_title: String::new(),
            untitled_recovery,
            mic_cache: None,
            spare_pick: (None, None),
            print: Default::default(),
        };
        if let Some(p) = open {
            app.open_path(&p);
        }
        app
    }

    // ------------------------------------------------------------------
    // Project actions

    pub fn new_project(&mut self) {
        self.request(After::New);
    }

    /// The demo musical, as an untitled project.
    pub fn open_demo(&mut self) {
        self.request(After::Demo);
    }

    /// The demo band show (song library and two setlists), as an untitled project.
    pub fn open_demo_band(&mut self) {
        self.request(After::DemoBand);
    }

    /// Open a project file (asks first when there are unsaved changes).
    pub fn open(&mut self, path: Option<PathBuf>) {
        self.request(After::Open(path));
    }

    pub fn close_project(&mut self) {
        self.request(After::Close);
    }

    fn request(&mut self, after: After) {
        if self.doc.as_ref().is_some_and(|d| d.dirty) {
            self.modal = Some(Modal::Unsaved(after));
        } else {
            self.run(after);
        }
    }

    fn run(&mut self, after: After) {
        match after {
            After::New => {
                self.set_doc(Document::new(Project::default()));
                self.page = Page::Production;
            }
            After::Demo => {
                self.set_doc(Document::new(bp_core::demo::musical()));
                self.page = Page::Mic(MicSheet::Plot);
            }
            After::DemoBand => {
                let p = bp_core::demo::band();
                let first = p.setlists.first().map(|s| s.id);
                self.set_doc(Document::new(p));
                self.page = first.map_or(Page::Songs, Page::Setlist);
            }
            After::Open(Some(path)) => self.open_path(&path),
            After::Open(None) => {
                if let Some(path) = self.pick_open() {
                    self.open_path(&path);
                }
            }
            After::Close => {
                self.discard_untitled_autosave();
                self.doc = None;
            }
            After::Quit => {
                self.allow_close = true;
            }
        }
    }

    fn set_doc(&mut self, doc: Document) {
        self.doc = Some(doc);
        self.tables.clear();
        self.last_autosave = Instant::now();
    }

    /// Open `path` directly (no unsaved-changes question).
    pub fn open_path(&mut self, path: &Path) {
        match Document::open(path) {
            Ok(doc) => {
                self.settings.remember(path);
                self.set_doc(doc);
                self.page = Page::Production;
                if let Some(p) = project::recoverable(path) {
                    if self.doc.as_ref().is_some_and(|d| d.project != p) {
                        self.modal = Some(Modal::Recover {
                            path: Some(path.to_path_buf()),
                            project: Box::new(p),
                        });
                    }
                }
            }
            Err(e) => self.modal = Some(Modal::Message(format!("Could not open the project.\n\n{e}"))),
        }
    }

    /// Continue the untitled project that was not saved.
    pub fn recover_untitled(&mut self) {
        if let Some(p) = self.untitled_recovery.take() {
            let mut doc = Document::new(p);
            doc.dirty = true;
            self.set_doc(doc);
        }
    }

    /// Save; asks for a file when the project has none yet. True when saved.
    pub fn save(&mut self) -> bool {
        let has_path = self.doc.as_ref().is_some_and(|d| d.path.is_some());
        if has_path {
            self.save_to(None)
        } else {
            self.save_as()
        }
    }

    pub fn save_as(&mut self) -> bool {
        match self.pick_save() {
            Some(p) => self.save_to(Some(&p)),
            None => false,
        }
    }

    /// Save to `path` (or the current file).
    pub fn save_to(&mut self, path: Option<&Path>) -> bool {
        let Some(doc) = self.doc.as_mut() else {
            return false;
        };
        let was_untitled = doc.path.is_none();
        match doc.save(path) {
            Ok(saved) => {
                self.settings.remember(&saved);
                if was_untitled {
                    self.discard_untitled_autosave();
                }
                self.notify(format!("Saved {}", saved.display()), false);
                true
            }
            Err(e) => {
                self.modal = Some(Modal::Message(format!("Could not save.\n\n{e}")));
                false
            }
        }
    }

    fn discard_untitled_autosave(&mut self) {
        if self.doc.as_ref().is_some_and(|d| d.path.is_none()) {
            if let Some(p) = settings::untitled_autosave() {
                let _ = std::fs::remove_file(p);
            }
        }
    }

    fn autosave(&mut self) {
        let Some(doc) = self.doc.as_ref() else { return };
        if !doc.dirty || self.last_autosave.elapsed() < AUTOSAVE_EVERY {
            return;
        }
        self.last_autosave = Instant::now();
        let r = match &doc.path {
            Some(p) => project::autosave(&doc.project, p),
            None => match settings::untitled_autosave() {
                Some(p) => project::to_string(&doc.project)
                    .and_then(|t| project::atomic_write(&p, t.as_bytes())),
                None => Ok(()),
            },
        };
        if let Err(e) = r {
            self.notify(format!("Autosave failed: {e}"), true);
        }
    }

    pub fn undo(&mut self) {
        if let Some(d) = self.doc.as_mut() {
            d.undo();
        }
    }

    pub fn redo(&mut self) {
        if let Some(d) = self.doc.as_mut() {
            d.redo();
        }
    }

    pub fn notify(&mut self, text: String, error: bool) {
        self.toast = Some((text, error, Instant::now()));
    }

    fn pick_open(&self) -> Option<PathBuf> {
        if !self.dialogs {
            return None;
        }
        let mut d = rfd::FileDialog::new()
            .set_title("Open project")
            .add_filter("Blueprint project", &[project::EXTENSION]);
        if let Some(dir) = &self.settings.last_dir {
            d = d.set_directory(dir);
        }
        d.pick_file()
    }

    fn pick_save(&self) -> Option<PathBuf> {
        if !self.dialogs {
            return None;
        }
        let name = self
            .doc
            .as_ref()
            .map(|d| d.title())
            .unwrap_or_else(|| "Untitled".into());
        let mut d = rfd::FileDialog::new()
            .set_title("Save project")
            .add_filter("Blueprint project", &[project::EXTENSION])
            .set_file_name(format!("{name}.{}", project::EXTENSION));
        if let Some(dir) = &self.settings.last_dir {
            d = d.set_directory(dir);
        }
        d.save_file().map(|p| project::with_extension(&p))
    }

    // ------------------------------------------------------------------
    // Frame

    /// Everything that is not drawing: shortcuts, autosave, title, closing.
    pub fn logic(&mut self, ctx: &egui::Context) {
        let cmd = |key| KeyboardShortcut::new(Modifiers::COMMAND, key);
        let cmd_shift = |key| KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, key);
        let (save_as, save, open, new) = ctx.input_mut(|i| {
            (
                i.consume_shortcut(&cmd_shift(Key::S)),
                i.consume_shortcut(&cmd(Key::S)),
                i.consume_shortcut(&cmd(Key::O)),
                i.consume_shortcut(&cmd(Key::N)),
            )
        });
        if self.modal.is_none() {
            if save_as && self.doc.is_some() {
                self.save_as();
            } else if save && self.doc.is_some() {
                self.save();
            }
            if open {
                self.open(None);
            }
            if new {
                self.new_project();
            }
            // Undo / redo, unless a text field handles them itself.
            if !ctx.text_edit_focused() {
                let (redo, undo) = ctx.input_mut(|i| {
                    (
                        i.consume_shortcut(&cmd_shift(Key::Z)) || i.consume_shortcut(&cmd(Key::Y)),
                        i.consume_shortcut(&cmd(Key::Z)),
                    )
                });
                if redo {
                    self.redo();
                } else if undo {
                    self.undo();
                }
            }
        }
        self.autosave();

        let title = match &self.doc {
            Some(d) => format!("{}{} - Blueprint", d.title(), if d.dirty { " *" } else { "" }),
            None => "Blueprint".to_string(),
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }

        // Closing the window with unsaved changes asks first.
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && self.doc.as_ref().is_some_and(|d| d.dirty)
        {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            if self.modal.is_none() {
                self.modal = Some(Modal::Unsaved(After::Quit));
            }
        }
        if self.allow_close {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if self.doc.as_ref().is_some_and(|d| d.dirty) {
            ctx.request_repaint_after(AUTOSAVE_EVERY);
        }
    }

    pub fn ui_root(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if self.doc.is_none() {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(theme::BG).inner_margin(egui::Margin::same(14)))
                .show(ui, |ui| self.start_page(ui));
        } else {
            egui::Panel::top("top_bar")
                .frame(egui::Frame::NONE.fill(theme::PANEL).inner_margin(egui::Margin::symmetric(12, 8)))
                .show(ui, |ui| self.top_bar(ui));
            egui::Panel::left("nav")
                .resizable(false)
                .exact_size(210.0)
                .frame(egui::Frame::NONE.fill(theme::PANEL).inner_margin(egui::Margin::symmetric(10, 12)))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.nav(ui));
                });
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(theme::BG).inner_margin(egui::Margin::symmetric(18, 14)))
                .show(ui, |ui| match self.page {
                    Page::Production => self.production_page(ui),
                    Page::Sheet(s) => self.sheet_page(ui, s),
                    Page::Mic(s) => self.mic_page(ui, s),
                    Page::Checks => self.checks_page(ui),
                    Page::Print => self.print_page(ui),
                    Page::Songs => self.songs_page(ui),
                    Page::Setlist(id) => self.setlist_page(ui, id),
                });
        }
        self.modals(&ctx);
        self.draw_toast(&ctx);
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Blueprint").strong().color(theme::BRAND).size(16.0));
            ui.separator();
            let (title, dirty, can_undo, can_redo) = match &self.doc {
                Some(d) => (d.title(), d.dirty, d.can_undo(), d.can_redo()),
                None => return,
            };
            ui.label(RichText::new(title).strong().size(15.0));
            if dirty {
                ui.label(RichText::new("unsaved").small().color(theme::WARN));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").on_hover_text("Close the project").clicked() {
                    self.close_project();
                }
                if ui.button("Save").on_hover_text("Save (Cmd/Ctrl+S)").clicked() {
                    self.save();
                }
                if ui.button("Open…").on_hover_text("Open a project (Cmd/Ctrl+O)").clicked() {
                    self.open(None);
                }
                ui.separator();
                if ui
                    .add_enabled(can_redo, egui::Button::new("Redo"))
                    .on_hover_text("Redo (Cmd/Ctrl+Shift+Z)")
                    .clicked()
                {
                    self.redo();
                }
                if ui
                    .add_enabled(can_undo, egui::Button::new("Undo"))
                    .on_hover_text("Undo (Cmd/Ctrl+Z)")
                    .clicked()
                {
                    self.undo();
                }
            });
        });
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        let section = |ui: &mut egui::Ui, t: &str| {
            ui.add_space(8.0);
            ui.label(RichText::new(t).small().color(theme::MUTED));
        };
        let mut item = |ui: &mut egui::Ui, page: Page, label: String| {
            let on = self.page == page;
            let w = ui.available_width();
            if ui
                .add(egui::Button::selectable(on, label).min_size(egui::vec2(w, 26.0)))
                .clicked()
            {
                self.page = page;
            }
        };
        section(ui, "PRODUCTION");
        item(ui, Page::Production, "Production".into());
        section(ui, "MASTER DATA");
        for s in Sheet::ALL {
            let n = self.doc.as_ref().map(|d| s.len(&d.project)).unwrap_or(0);
            item(ui, Page::Sheet(s), format!("{}  ({n})", s.title()));
        }
        section(ui, "MIC PLAN");
        for s in MicSheet::ALL {
            item(ui, Page::Mic(s), s.title().to_string());
        }
        section(ui, "");
        let n = self.mic_view().problems.len();
        let on = self.page == Page::Checks;
        let label = if n == 0 {
            RichText::new("Checks: all fine")
        } else {
            RichText::new(format!("Checks: {n} problem{}", if n == 1 { "" } else { "s" })).color(theme::WARN)
        };
        let w = ui.available_width();
        if ui.add(egui::Button::selectable(on, label).min_size(egui::vec2(w, 28.0))).clicked() {
            self.page = Page::Checks;
        }
        let on = self.page == Page::Print;
        if ui
            .add(egui::Button::selectable(on, RichText::new("Print / PDF").strong()).min_size(egui::vec2(w, 30.0)))
            .clicked()
        {
            self.page = Page::Print;
        }
        section(ui, "SETLISTS");
        let lists: Vec<(Id, String)> = self
            .doc
            .as_ref()
            .map(|d| {
                d.project
                    .setlists
                    .iter()
                    .map(|s| (s.id, if s.name.trim().is_empty() { "Untitled setlist".into() } else { s.name.clone() }))
                    .collect()
            })
            .unwrap_or_default();
        let songs = self.doc.as_ref().map_or(0, |d| d.project.songs.len());
        let mut item = |ui: &mut egui::Ui, page: Page, label: String| {
            let on = self.page == page;
            let w = ui.available_width();
            if ui.add(egui::Button::selectable(on, label).min_size(egui::vec2(w, 26.0))).clicked() {
                self.page = page;
            }
        };
        item(ui, Page::Songs, format!("Song library  ({songs})"));
        for (id, name) in lists {
            item(ui, Page::Setlist(id), name);
        }
        if ui
            .add(egui::Button::new(RichText::new("+ New setlist").color(theme::MUTED)).frame(false))
            .clicked()
        {
            self.new_setlist();
        }
        section(ui, "MORE PLANS");
        for p in ["Patch", "Stage plot", "Run of show", "More plans"] {
            ui.add_enabled(false, egui::Button::selectable(false, p).min_size(egui::vec2(ui.available_width(), 26.0)))
                .on_disabled_hover_text("Coming in a later version");
        }
    }

    fn modals(&mut self, ctx: &egui::Context) {
        let Some(modal) = self.modal.take() else { return };
        let mut keep = true;
        let mut then: Option<After> = None;
        egui::Modal::new(egui::Id::unique("app_modal")).show(ctx, |ui| {
            ui.set_max_width(440.0);
            match &modal {
                Modal::Unsaved(after) => {
                    ui.heading("Unsaved changes");
                    ui.label("Save the changes to this project first?");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Save").strong()).clicked() {
                            keep = false;
                            if self.save() {
                                then = Some(after.clone());
                            }
                        }
                        if ui.button("Don't save").clicked() {
                            keep = false;
                            self.discard_untitled_autosave();
                            if let Some(d) = self.doc.as_mut() {
                                d.dirty = false;
                                if let Some(p) = &d.path {
                                    let _ = std::fs::remove_file(project::autosave_path(p));
                                }
                            }
                            then = Some(after.clone());
                        }
                        if ui.button("Cancel").clicked() {
                            keep = false;
                        }
                    });
                }
                Modal::Recover { path, project } => {
                    ui.heading("Recover unsaved changes?");
                    ui.label(match path {
                        Some(p) => format!(
                            "Blueprint was not closed properly. There are newer, unsaved changes to {}.",
                            p.display()
                        ),
                        None => "There are unsaved changes from last time.".into(),
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Recover").strong()).clicked() {
                            keep = false;
                            if let Some(d) = self.doc.as_mut() {
                                let p = (**project).clone();
                                d.edit("recover", |x| *x = p);
                            }
                        }
                        if ui.button("Keep the saved file").clicked() {
                            keep = false;
                            if let Some(p) = path {
                                let _ = std::fs::remove_file(project::autosave_path(p));
                            }
                        }
                    });
                }
                Modal::Message(text) => {
                    ui.label(text);
                    ui.add_space(8.0);
                    if ui.button("OK").clicked() {
                        keep = false;
                    }
                }
            }
        });
        if keep && self.modal.is_none() {
            self.modal = Some(modal);
        }
        if let Some(after) = then {
            self.run(after);
        }
    }

    fn draw_toast(&mut self, ctx: &egui::Context) {
        let Some((text, error, since)) = &self.toast else { return };
        let ttl = if *error { 8.0 } else { 3.0 };
        if since.elapsed().as_secs_f32() > ttl {
            self.toast = None;
            return;
        }
        let (text, error) = (text.clone(), *error);
        egui::Area::new(egui::Id::unique("toast"))
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -18.0])
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(if error { Color32::from_rgb(80, 22, 22) } else { theme::CARD })
                    .stroke(egui::Stroke::new(1.0, if error { theme::ERROR } else { theme::LINE }))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        ui.set_max_width(640.0);
                        ui.label(text);
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(250));
    }

    /// Whether a modal dialog is open (tests).
    pub fn modal_open(&self) -> bool {
        self.modal.is_some()
    }
}
