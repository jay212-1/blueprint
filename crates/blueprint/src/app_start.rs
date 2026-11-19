//! The start screen over a blueprint backdrop: new project (a folder with
//! the project file), open, recently opened, recover, and the demos in the
//! corner. Same layout as the start screens of Show Master and Lantern, in
//! Blueprint's blue.

use crate::app::App;
use crate::theme;
use egui::{pos2, Align, Color32, CornerRadius, Frame, Layout, Margin, Rect, RichText, Sense, Stroke, TextureHandle, Vec2};
use std::path::{Path, PathBuf};

/// The new-project form of the start screen.
#[derive(Default)]
pub struct StartState {
    /// The "New project" form is open.
    pub form: bool,
    pub name: String,
    /// The folder the project folder goes into.
    pub location: PathBuf,
    pub error: Option<String>,
    banner: Option<TextureHandle>,
}

const BLUE: Color32 = theme::BRAND;
const TEXT: Color32 = theme::TEXT;
const MUTED: Color32 = theme::MUTED;
const LINE: Color32 = Color32::from_rgb(44, 49, 58);
/// A card on the start screen: dark glass, a stroke that lights up on hover.
const GLASS: Color32 = Color32::from_rgba_premultiplied(14, 17, 23, 228);
/// Ink on the blue buttons.
const ON_BLUE: Color32 = Color32::from_rgb(4, 16, 32);
/// Most projects shown under "Recent".
const RECENT: usize = 8;

/// Where new projects go unless another place is chosen: Documents/Blueprint.
pub fn default_location() -> PathBuf {
    dirs::document_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Documents")))
        .map(|d| d.join("Blueprint"))
        .unwrap_or_default()
}

/// A project name usable as a folder and file name.
pub fn clean_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .trim()
        .to_string()
}

impl StartState {
    /// The file a new project gets: `<location>/<name>/<name>.blueprint`.
    pub fn project_file(&self) -> PathBuf {
        let n = clean_name(&self.name);
        self.location.join(&n).join(format!("{n}.{}", bp_core::project::EXTENSION))
    }

    /// Why the form cannot be sent yet.
    pub fn problem(&self) -> Option<String> {
        if clean_name(&self.name).is_empty() {
            return Some("Give the project a name.".into());
        }
        if self.location.as_os_str().is_empty() {
            return Some("Choose where the project folder goes.".into());
        }
        let dir = self.project_file().parent().map(Path::to_path_buf)?;
        if dir.exists() {
            return Some(format!("{} exists already.", dir.display()));
        }
        None
    }
}

impl App {
    /// Open the new-project form on the start screen.
    pub(crate) fn open_new_form(&mut self) {
        self.start.form = true;
        self.start.error = None;
        self.start.name.clear();
        if self.start.location.as_os_str().is_empty() {
            self.start.location = self.settings.new_dir.clone().unwrap_or_else(default_location);
        }
    }

    /// Make the project of the form: its own folder with the project file,
    /// the production name filled in, saved at once.
    pub fn create_project(&mut self) {
        if let Some(p) = self.start.problem() {
            self.start.error = Some(p);
            return;
        }
        let path = self.start.project_file();
        if let Some(dir) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                self.start.error = Some(format!("Could not make the folder {}: {e}", dir.display()));
                return;
            }
        }
        let project = bp_core::model::Project {
            meta: bp_core::model::Meta { production: self.start.name.trim().to_string(), ..Default::default() },
            ..Default::default()
        };
        self.start_new_document(project);
        if self.save_to(Some(&path)) {
            self.start.form = false;
            self.start.error = None;
            self.settings.new_dir = Some(self.start.location.clone());
            self.settings.save();
        } else {
            self.doc = None;
            self.start.error = Some(format!("Could not save {}.", path.display()));
        }
    }

    // ------------------------------------------------------------ backdrop

    fn backdrop(&mut self, ui: &mut egui::Ui) -> Rect {
        let rect = ui.max_rect();
        let tex = self
            .start
            .banner
            .get_or_insert_with(|| ui.ctx().load_texture("bp_banner", banner_image(1280, 720), egui::TextureOptions::LINEAR));
        // Cover: fill the window, crop what is too much; keep the right side.
        let [tw, th] = tex.size().map(|v| v as f32);
        let scale = (rect.width() / tw).max(rect.height() / th);
        let size = Vec2::new(tw * scale, th * scale);
        let min = pos2(rect.right() - size.x, rect.center().y - size.y / 2.0);
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        let p = ui.painter();
        p.rect_filled(rect, 0.0, theme::BG);
        p.image(tex.id(), Rect::from_min_size(min, size), uv, Color32::WHITE);
        rect
    }

    // ------------------------------------------------------------ start screen

    pub(crate) fn start_page(&mut self, ui: &mut egui::Ui) {
        let rect = self.backdrop(ui);
        let pad = (rect.width() * 0.06).clamp(28.0, 96.0);
        let inner = rect.shrink2(Vec2::new(pad, pad * 0.8));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(Layout::top_down(Align::Min)));
        let ui = &mut child;

        ui.add_space((inner.height() * 0.05).min(36.0));
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
            blueprint_mark(ui.painter(), r);
            ui.add_space(8.0);
            ui.label(RichText::new("Blueprint").size(46.0).strong().color(TEXT));
        });
        ui.add_space(2.0);
        ui.label(RichText::new("Planning for live shows").size(18.0).color(Color32::from_rgb(196, 200, 210)));
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "Version {}  ·  Mic plots, setlists, patch, stage plots  ·  Print-ready PDF",
                env!("CARGO_PKG_VERSION")
            ))
            .size(12.5)
            .color(MUTED),
        );
        ui.add_space((inner.height() * 0.06).clamp(16.0, 48.0));

        if self.start.form {
            self.new_form(ui);
        } else {
            if let Some(e) = self.start.error.clone() {
                notice(ui, theme::ERROR, &e);
                ui.add_space(10.0);
            }
            self.start_home(ui);
        }

        // The foot: dropping files on the left, the demos on the right.
        let y = rect.bottom() - pad * 0.45;
        ui.painter().text(
            pos2(inner.left(), y),
            egui::Align2::LEFT_CENTER,
            "Drop a .blueprint file anywhere to open it",
            egui::FontId::proportional(12.5),
            MUTED,
        );
        let demos = Rect::from_min_max(pos2(inner.center().x, y - 12.0), pos2(inner.right(), y + 12.0));
        let mut foot = ui.new_child(egui::UiBuilder::new().max_rect(demos).layout(Layout::right_to_left(Align::Center)));
        foot.spacing_mut().item_spacing.x = 14.0;
        let link = |ui: &mut egui::Ui, text: &str, hint: &str| {
            ui.add(egui::Button::new(RichText::new(text).size(12.5).color(Color32::from_rgb(170, 190, 215))).frame(false))
                .on_hover_text(hint)
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
        };
        if link(&mut foot, "Demo band show", "\"Northbound Static\": songs, setlists, a full patch, stage plot and more plans") {
            self.open_demo_band();
        }
        if link(&mut foot, "Demo musical", "\"The Lighthouse Keeper\": 12 performers, 15 scenes, 16 transmitters, swaps and casts of the day") {
            self.open_demo();
        }
        foot.label(RichText::new("Try:").size(12.5).color(MUTED));
    }

    fn start_home(&mut self, ui: &mut egui::Ui) {
        let gap = 16.0;
        let w = ((ui.available_width() - 2.0 * gap) / 3.0).clamp(190.0, 300.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            if action_card(ui, w, Icon::New, "New project", "A folder for the production with its project file. Prints and exports go there too.", true)
                .clicked()
            {
                self.open_new_form();
            }
            if action_card(ui, w, Icon::Open, "Open project", "A .blueprint file from this computer, a USB stick or a shared folder.", false).clicked() {
                self.open(None);
            }
            if self.untitled_recovery.is_some()
                && action_card(ui, w, Icon::Recover, "Recover", "An untitled project was not saved last time. Continue where it stopped.", false)
                    .clicked()
            {
                self.recover_untitled();
            }
        });

        let recent: Vec<PathBuf> = self.settings.recent.iter().take(RECENT).cloned().collect();
        if recent.is_empty() {
            return;
        }
        ui.add_space(28.0);
        let width = 3.0 * w + 2.0 * gap;
        ui.horizontal(|ui| {
            ui.set_width(width);
            ui.label(RichText::new("RECENT PROJECTS").size(11.5).strong().color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(egui::Button::new(RichText::new("Clear list").size(11.5).color(MUTED)).frame(false)).clicked() {
                    self.settings.recent.clear();
                    self.settings.save();
                }
            });
        });
        ui.add_space(6.0);
        let mut open = None;
        Frame::new()
            .fill(GLASS)
            .stroke(Stroke::new(1.0, LINE))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::same(6))
            .show(ui, |ui| {
                ui.set_width(width - 12.0);
                let rows = ((ui.available_height() - 40.0) / 44.0).floor().max(1.0) as usize;
                for p in recent.iter().take(rows) {
                    if recent_row(ui, p).clicked() {
                        open = Some(p.clone());
                    }
                }
            });
        if let Some(p) = open {
            if p.is_file() {
                self.start.error = None;
                self.open(Some(p));
            } else {
                self.start.error = Some(format!("Not there any more: {}", p.display()));
            }
        }
    }

    fn new_form(&mut self, ui: &mut egui::Ui) {
        let dialogs = self.dialogs;
        let mut go = false;
        form_card(ui, |ui| {
            ui.label(RichText::new("New project").size(24.0).strong().color(TEXT));
            ui.label(
                RichText::new(
                    "Blueprint makes a folder for the production with the project file in it. Keep logos, \
                     imported lists and the exported PDFs there too, so the folder can be backed up or \
                     handed on as one piece.",
                )
                .color(MUTED),
            );
            ui.add_space(16.0);
            let st = &mut self.start;
            egui::Grid::new("bp_new_project").num_columns(2).spacing([16.0, 12.0]).min_col_width(110.0).show(ui, |ui| {
                let l = ui.label(RichText::new("Production").color(MUTED));
                let r = ui.add(egui::TextEdit::singleline(&mut st.name).hint_text("e.g. Summer Tour 2027").desired_width(360.0)).labelled_by(l.id);
                if st.name.is_empty() && !r.has_focus() && !r.lost_focus() {
                    r.request_focus();
                }
                ui.end_row();
                ui.label(RichText::new("Location").color(MUTED));
                ui.horizontal(|ui| {
                    path_field(ui, &st.location, "No place chosen");
                    if ui.button("Choose…").clicked() && dialogs {
                        let mut d = rfd::FileDialog::new();
                        if st.location.is_dir() {
                            d = d.set_directory(&st.location);
                        }
                        if let Some(p) = d.pick_folder() {
                            st.location = p;
                        }
                    }
                });
                ui.end_row();
            });
            ui.add_space(10.0);
            if !clean_name(&st.name).is_empty() && !st.location.as_os_str().is_empty() {
                ui.label(RichText::new(format!("Creates  {}", st.project_file().display())).size(12.5).color(MUTED));
            }
            let problem = st.problem();
            ui.add_space(14.0);
            if let Some(e) = &st.error {
                ui.label(RichText::new(e).color(theme::ERROR));
                ui.add_space(6.0);
            }
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(RichText::new("Cancel").size(15.0)).corner_radius(8).min_size(Vec2::new(110.0, 38.0)))
                    .clicked()
                {
                    st.form = false;
                    st.error = None;
                }
                ui.add_space(6.0);
                go = ui
                    .add_enabled(
                        problem.is_none(),
                        egui::Button::new(RichText::new("Create project").size(15.0).strong().color(ON_BLUE))
                            .fill(BLUE)
                            .corner_radius(8)
                            .min_size(Vec2::new(180.0, 38.0)),
                    )
                    .clicked();
                if let Some(p) = &problem {
                    ui.add_space(8.0);
                    ui.label(RichText::new(p).color(MUTED));
                }
            });
            go |= problem.is_none() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                st.form = false;
                st.error = None;
            }
        });
        if go {
            self.create_project();
        }
    }
}

// ------------------------------------------------------------ widgets

fn notice(ui: &mut egui::Ui, color: Color32, text: &str) {
    Frame::new()
        .fill(color.gamma_multiply(0.12))
        .stroke(Stroke::new(1.0, color))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(TEXT));
        });
}

#[derive(Clone, Copy)]
enum Icon {
    New,
    Open,
    Recover,
}

fn action_card(ui: &mut egui::Ui, w: f32, icon: Icon, title: &str, about: &str, primary: bool) -> egui::Response {
    let h = 168.0;
    let (rect, r) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, title));
    let hot = r.hovered();
    let p = ui.painter();
    let stroke = if primary {
        BLUE.gamma_multiply(if hot { 1.0 } else { 0.7 })
    } else if hot {
        Color32::from_rgb(90, 98, 112)
    } else {
        LINE
    };
    p.rect(
        rect,
        CornerRadius::same(14),
        if hot { Color32::from_rgba_premultiplied(24, 28, 36, 236) } else { GLASS },
        Stroke::new(if primary { 1.5 } else { 1.0 }, stroke),
        egui::StrokeKind::Inside,
    );
    let tile = Rect::from_min_size(rect.min + Vec2::new(20.0, 20.0), Vec2::splat(40.0));
    p.rect_filled(tile, CornerRadius::same(10), if primary { BLUE } else { Color32::from_rgb(40, 45, 54) });
    let ink = if primary { ON_BLUE } else { TEXT };
    draw_icon(p, tile.shrink(11.0), icon, ink);
    p.text(rect.min + Vec2::new(20.0, 78.0), egui::Align2::LEFT_TOP, title, egui::FontId::proportional(18.0), TEXT);
    let galley = p.layout(about.to_string(), egui::FontId::proportional(13.0), MUTED, w - 40.0);
    p.galley(rect.min + Vec2::new(20.0, 106.0), galley, MUTED);
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn draw_icon(p: &egui::Painter, r: Rect, icon: Icon, ink: Color32) {
    let s = Stroke::new(2.2, ink);
    match icon {
        Icon::New => {
            p.line_segment([r.center_top(), r.center_bottom()], s);
            p.line_segment([r.left_center(), r.right_center()], s);
        }
        Icon::Open => {
            let body = Rect::from_min_max(r.min + Vec2::new(0.0, 4.0), r.max);
            p.rect_stroke(body, CornerRadius::same(2), s, egui::StrokeKind::Middle);
            p.line_segment([r.min + Vec2::new(0.0, 4.0), r.min + Vec2::new(6.0, 0.0)], s);
            p.line_segment([r.min + Vec2::new(6.0, 0.0), r.min + Vec2::new(10.0, 4.0)], s);
        }
        Icon::Recover => {
            // A circular arrow.
            let c = r.center();
            let rad = r.width() / 2.0;
            let pts: Vec<egui::Pos2> = (0..=20)
                .map(|i| {
                    let a = std::f32::consts::PI * (0.25 + 1.5 * i as f32 / 20.0);
                    pos2(c.x + rad * a.cos(), c.y - rad * a.sin())
                })
                .collect();
            let end = pts[pts.len() - 1];
            p.add(egui::Shape::line(pts, s));
            p.line_segment([end, end + Vec2::new(5.0, 0.0)], s);
            p.line_segment([end, end + Vec2::new(0.0, -5.0)], s);
        }
    }
}

/// The app icon: a blue sheet with a grid and a stage drawn on it.
fn blueprint_mark(p: &egui::Painter, r: Rect) {
    p.rect(r, CornerRadius::same(10), Color32::from_rgb(20, 23, 28), Stroke::new(1.0, Color32::from_rgb(42, 47, 56)), egui::StrokeKind::Inside);
    let u = r.width() / 1024.0;
    let at = |x: f32, y: f32| pos2(r.left() + x * u, r.top() + y * u);
    let sheet = Rect::from_min_max(at(232.0, 232.0), at(792.0, 792.0));
    p.rect_filled(sheet, CornerRadius::same(4), Color32::from_rgb(29, 95, 160));
    let grid = Stroke::new(0.8, Color32::from_rgb(56, 152, 236));
    for k in 1..5 {
        let v = 232.0 + 112.0 * k as f32;
        p.line_segment([at(232.0, v), at(792.0, v)], grid);
        p.line_segment([at(v, 232.0), at(v, 792.0)], grid);
    }
    let ink = Stroke::new(2.4, Color32::from_rgb(234, 243, 253));
    p.add(egui::Shape::line(vec![at(312.0, 640.0), at(312.0, 400.0), at(712.0, 400.0), at(712.0, 640.0)], ink));
    let arc: Vec<egui::Pos2> = (0..=12)
        .map(|i| {
            let t = i as f32 / 12.0;
            let x = 312.0 + 400.0 * t;
            let y = 640.0 + 160.0 * t * (1.0 - t);
            at(x, y)
        })
        .collect();
    p.add(egui::Shape::line(arc, ink));
    p.circle_filled(at(512.0, 520.0), 40.0 * u, Color32::from_rgb(234, 243, 253));
}

fn recent_row(ui: &mut egui::Ui, p: &Path) -> egui::Response {
    let w = ui.available_width();
    let (rect, r) = ui.allocate_exact_size(Vec2::new(w, 44.0), Sense::click());
    let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
    let there = p.is_file();
    let painter = ui.painter();
    if r.hovered() {
        painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(30, 35, 44));
    }
    let dot = pos2(rect.left() + 18.0, rect.center().y);
    painter.circle_filled(dot, 4.0, if there { BLUE } else { LINE });
    painter.text(
        pos2(rect.left() + 34.0, rect.center().y - 8.0),
        egui::Align2::LEFT_CENTER,
        &name,
        egui::FontId::proportional(15.0),
        if there { TEXT } else { MUTED },
    );
    let place = p.parent().map(|d| d.display().to_string()).unwrap_or_default();
    let place = if there { place } else { format!("Missing - {place}") };
    painter.text(pos2(rect.left() + 34.0, rect.center().y + 9.0), egui::Align2::LEFT_CENTER, place, egui::FontId::proportional(12.0), MUTED);
    let when = std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%d %b %Y, %H:%M").to_string())
        .unwrap_or_default();
    painter.text(pos2(rect.right() - 14.0, rect.center().y), egui::Align2::RIGHT_CENTER, when, egui::FontId::proportional(12.0), MUTED);
    r.on_hover_text(p.display().to_string()).on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn path_field(ui: &mut egui::Ui, p: &Path, empty: &str) {
    let (text, color) = if p.as_os_str().is_empty() { (empty.to_string(), MUTED) } else { (p.display().to_string(), TEXT) };
    Frame::new()
        .fill(theme::BG)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.set_width(300.0);
            ui.add(egui::Label::new(RichText::new(text).color(color)).truncate());
        })
        .response
        .on_hover_text(p.display().to_string());
}

fn form_card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(Color32::from_rgba_premultiplied(16, 19, 25, 242))
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::symmetric(28, 24))
        .show(ui, |ui| {
            ui.set_max_width(640.0);
            add(ui);
        });
}

/// The backdrop: a blueprint sheet in the dark, its grid fading in to the
/// right, a stage plan drawn in light lines with a glow, light from above.
pub fn banner_image(w: usize, h: usize) -> egui::ColorImage {
    fn lin(c: [f32; 3]) -> [f32; 3] {
        c.map(|v| (v / 255.0).powf(2.2))
    }
    fn smooth(a: f32, b: f32, x: f32) -> f32 {
        let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
    /// Distance to the segment a-b.
    fn seg(px: f32, py: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let t = (((px - a.0) * dx + (py - a.1) * dy) / (dx * dx + dy * dy).max(1e-9)).clamp(0.0, 1.0);
        ((px - a.0 - t * dx).powi(2) + (py - a.1 - t * dy).powi(2)).sqrt()
    }
    let top = lin([12.0, 17.0, 26.0]);
    let bottom = lin([4.0, 5.0, 8.0]);
    let sheet = lin([16.0, 52.0, 96.0]);
    let blue = lin([56.0, 152.0, 236.0]);
    let ink = lin([210.0, 232.0, 255.0]);
    let a = w as f32 / h as f32;
    // The drawing, x in units of the height: a stage, a riser, a thrust,
    // wedges and mic positions, dimension lines.
    let x0 = 0.60 * a;
    let (sx0, sx1, sy0, sy1) = (x0, x0 + 0.62, 0.22, 0.66);
    /// From, to, brightness.
    type Line = ((f32, f32), (f32, f32), f32);
    let mut lines: Vec<Line> = vec![
        ((sx0, sy0), (sx1, sy0), 1.0),
        ((sx0, sy0), (sx0, sy1), 1.0),
        ((sx1, sy0), (sx1, sy1), 1.0),
        ((sx0, sy1), (sx1, sy1), 1.0),
        // Riser.
        ((sx0 + 0.22, sy0 + 0.05), (sx0 + 0.40, sy0 + 0.05), 0.7),
        ((sx0 + 0.22, sy0 + 0.05), (sx0 + 0.22, sy0 + 0.17), 0.7),
        ((sx0 + 0.40, sy0 + 0.05), (sx0 + 0.40, sy0 + 0.17), 0.7),
        ((sx0 + 0.22, sy0 + 0.17), (sx0 + 0.40, sy0 + 0.17), 0.7),
        // Dimension line under the stage with its ticks.
        ((sx0, sy1 + 0.07), (sx1, sy1 + 0.07), 0.45),
        ((sx0, sy1 + 0.05), (sx0, sy1 + 0.09), 0.45),
        ((sx1, sy1 + 0.05), (sx1, sy1 + 0.09), 0.45),
        // And at the side.
        ((sx1 + 0.06, sy0), (sx1 + 0.06, sy1), 0.45),
        ((sx1 + 0.04, sy0), (sx1 + 0.08, sy0), 0.45),
        ((sx1 + 0.04, sy1), (sx1 + 0.08, sy1), 0.45),
    ];
    // Wedges along the front.
    for k in 0..3 {
        let cx = sx0 + 0.14 + 0.17 * k as f32;
        let (y0, y1) = (sy1 - 0.08, sy1 - 0.03);
        lines.push(((cx - 0.035, y0), (cx + 0.035, y0), 0.8));
        lines.push(((cx - 0.035, y0), (cx - 0.025, y1), 0.8));
        lines.push(((cx + 0.035, y0), (cx + 0.025, y1), 0.8));
        lines.push(((cx - 0.025, y1), (cx + 0.025, y1), 0.8));
    }
    // Mic positions: rings with a dot, the middle one lit.
    let mics = [(sx0 + 0.14, sy1 - 0.16, 0.6), (sx0 + 0.31, sy1 - 0.18, 1.0), (sx0 + 0.48, sy1 - 0.16, 0.6)];
    // Light from above: (origin x of the width, angle, spread, strength).
    let beams = [(0.72, 0.16, 0.07, 0.40), (0.92, -0.25, 0.06, 0.28), (0.52, 0.42, 0.05, 0.22)];
    let mut pixels = Vec::with_capacity(w * h);
    for py in 0..h {
        let v = py as f32 / h as f32;
        for px in 0..w {
            let u = px as f32 / w as f32;
            let x = u * a;
            let base = smooth(0.0, 1.0, v);
            let mut c = [0.0f32; 3];
            for k in 0..3 {
                c[k] = top[k] + (bottom[k] - top[k]) * base;
            }
            // The sheet: blue paper that fades in towards the right.
            let paper = smooth(0.30, 0.75, u) * (1.0 - 0.55 * smooth(0.55, 1.0, v));
            for k in 0..3 {
                c[k] += sheet[k] * paper * 0.55;
            }
            // Its grid, fine and coarse.
            let fine = 0.025;
            let gx = ((x / fine).fract() - 0.5).abs();
            let gy = ((v / fine).fract() - 0.5).abs();
            let gline = smooth(0.47, 0.5, gx.max(gy)) * 0.18;
            let coarse = fine * 5.0;
            let cx = ((x / coarse).fract() - 0.5).abs();
            let cy = ((v / coarse).fract() - 0.5).abs();
            let cline = smooth(0.485, 0.5, cx.max(cy)) * 0.35;
            for k in 0..3 {
                c[k] += blue[k] * (gline + cline) * paper * 0.5;
            }
            // Light from above.
            for (ox, ang, spread, kk) in beams {
                let dx = (u - ox) * a;
                let dy = v + 0.08;
                let an = dx.atan2(dy) - ang;
                let fall = (-(an / spread).powi(2)).exp();
                let len = (dx * dx + dy * dy).sqrt();
                let fade = (-len * 1.3).exp() * smooth(0.0, 0.12, dy);
                for k in 0..3 {
                    c[k] += blue[k] * fall * fade * kk * 0.35;
                }
            }
            // The drawing: thin light lines with a soft blue glow.
            let mut d_min = f32::MAX;
            let mut strength = 0.0f32;
            for (p0, p1, kk) in &lines {
                let d = seg(x, v, *p0, *p1);
                if d < d_min {
                    d_min = d;
                    strength = *kk;
                }
            }
            for (mx, my, kk) in mics {
                let r = ((x - mx).powi(2) + (v - my).powi(2)).sqrt();
                let ring = (r - 0.028).abs();
                if ring < d_min {
                    d_min = ring;
                    strength = kk;
                }
                let dot = (r - 0.008).max(0.0);
                if dot < d_min {
                    d_min = dot;
                    strength = kk;
                }
            }
            let core = smooth(0.0022, 0.0008, d_min);
            let glow = (-(d_min / 0.012).powi(2)).exp() * 0.25 + (-d_min / 0.06).exp() * 0.06;
            for k in 0..3 {
                c[k] += (ink[k] * core * 0.85 + blue[k] * glow) * strength;
            }
            // Darker towards the left so the text reads.
            let shade = 0.35 + 0.65 * smooth(0.0, 0.55, u);
            let rgb = c.map(|v| ((v * shade).max(0.0).powf(1.0 / 2.2) * 255.0).min(255.0) as u8);
            pixels.push(Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
        }
    }
    egui::ColorImage::new([w, h], pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_folders() {
        assert_eq!(clean_name("  Tour: 2027/28 "), "Tour- 2027-28");
        let st = StartState { name: "Summer Tour".into(), location: PathBuf::from("/tmp/x"), ..Default::default() };
        assert_eq!(st.project_file(), PathBuf::from("/tmp/x/Summer Tour/Summer Tour.blueprint"));
        assert!(StartState::default().problem().is_some());
    }
}
