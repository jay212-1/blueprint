//! The stage plot editor: the stage from above, drawn from the same shapes
//! as the print. Click to select, drag to move (snaps to the grid), arrows
//! nudge, R turns, Delete removes.

use crate::app::App;
use crate::app_setlist::parsed_field;
use crate::theme;
use bp_core::model::{Id, Project};
use bp_core::plans::PlanKind;
use bp_core::stage::{parse_channels, plot_shapes, snap, Fill, Shape, StageItem, StagePlot, Symbol};
use egui::{Color32, Key, Pos2, RichText, Sense, Stroke};

/// Selection and dragging on the stage plot.
#[derive(Default)]
pub struct StageState {
    /// Plan and item.
    pub selected: Option<(Id, Id)>,
    /// The item being dragged.
    drag: Option<Id>,
    /// Where the stage's corner (0, 0) is on screen and points per metre (tests).
    pub view: Option<(Pos2, f32)>,
}

const PAPER: Color32 = Color32::from_rgb(246, 247, 249);
const INK: Color32 = Color32::from_rgb(30, 33, 40);

fn fill_color(fill: Fill, tint: &str) -> Option<Color32> {
    let base = match fill {
        Fill::None => return None,
        Fill::Light => Color32::from_rgb(226, 229, 234),
        Fill::Mid => Color32::from_rgb(176, 182, 192),
        Fill::Dark => Color32::from_rgb(64, 70, 82),
        Fill::Accent => theme::ACCENT,
    };
    match theme::color(tint) {
        Some(t) if fill != Fill::Accent => Some(mix(base, t, if fill == Fill::Dark { 0.55 } else { 0.4 })),
        _ => Some(base),
    }
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

fn plot_of(p: &mut Project, plan: Id) -> Option<&mut StagePlot> {
    match p.plan_mut(plan).map(|x| &mut x.kind) {
        Some(PlanKind::StagePlot(s)) => Some(s),
        _ => None,
    }
}

impl App {
    pub(crate) fn stage_editor(&mut self, ui: &mut egui::Ui, plan: Id, s: &StagePlot) {
        // The symbol palette.
        let mut add = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Add").color(theme::MUTED));
            for sym in Symbol::ALL {
                if ui.small_button(sym.label()).clicked() {
                    add = Some(sym);
                }
            }
        });
        if let Some(sym) = add {
            self.stage_add(plan, s, sym);
        }
        let Some(doc) = self.doc.as_mut() else { return };
        let project = doc.project.clone();
        let s = match project.plan(plan).map(|x| &x.kind) {
            Some(PlanKind::StagePlot(s)) => s.clone(),
            _ => return,
        };
        let selected = self.stage.selected.filter(|(p, i)| *p == plan && s.items.iter().any(|x| x.id == *i)).map(|x| x.1);

        // Scale to fit the width and the height left.
        let margin = 26.0;
        let avail = ui.available_width().max(200.0) - 2.0 * margin;
        // Leave room for the items table below.
        let high = (ui.available_height() - 300.0).clamp(200.0, 520.0);
        let scale = (avail / s.width as f32).min(high / s.depth as f32).max(8.0);
        let size = egui::vec2(s.width as f32 * scale + 2.0 * margin, s.depth as f32 * scale + 2.0 * margin);
        let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let origin = resp.rect.min + egui::vec2(margin, margin);
        let to_screen = |x: f64, y: f64| Pos2::new(origin.x + x as f32 * scale, origin.y + y as f32 * scale);
        let to_stage = |p: Pos2| (((p.x - origin.x) / scale) as f64, ((p.y - origin.y) / scale) as f64);
        self.stage.view = Some((origin, scale));
        let stage = egui::Rect::from_min_size(origin, egui::vec2(s.width as f32 * scale, s.depth as f32 * scale));

        painter.rect_filled(stage, 2.0, PAPER);
        if s.show_grid && s.grid > 0.0 && s.grid * scale as f64 >= 6.0 {
            let grid = Stroke::new(1.0, Color32::from_rgb(222, 225, 231));
            let mut x = s.grid;
            while x < s.width - 1e-6 {
                painter.line_segment([to_screen(x, 0.0), to_screen(x, s.depth)], grid);
                x += s.grid;
            }
            let mut y = s.grid;
            while y < s.depth - 1e-6 {
                painter.line_segment([to_screen(0.0, y), to_screen(s.width, y)], grid);
                y += s.grid;
            }
        }
        painter.rect_stroke(stage, 2.0, Stroke::new(1.5, INK), egui::StrokeKind::Middle);
        if !s.front.trim().is_empty() {
            painter.text(
                Pos2::new(stage.center().x, stage.max.y + 13.0),
                egui::Align2::CENTER_CENTER,
                s.front.to_uppercase(),
                egui::FontId::proportional(12.0),
                theme::MUTED,
            );
        }
        painter.text(
            Pos2::new(stage.min.x, stage.min.y - 12.0),
            egui::Align2::LEFT_CENTER,
            format!("{} x {} m, grid {} m", fmt(s.width), fmt(s.depth), fmt(s.grid)),
            egui::FontId::proportional(11.0),
            theme::MUTED,
        );

        let ink = Stroke::new(1.2, INK);
        for shape in plot_shapes(&project, &s) {
            match shape {
                Shape::Poly { points, fill, tint } => {
                    let pts: Vec<Pos2> = points.iter().map(|(x, y)| to_screen(*x, *y)).collect();
                    let f = fill_color(fill, &tint).unwrap_or(Color32::TRANSPARENT);
                    painter.add(egui::Shape::convex_polygon(pts, f, ink));
                }
                Shape::Circle { x, y, r, fill, tint } => {
                    let c = to_screen(x, y);
                    let r = r as f32 * scale;
                    match fill_color(fill, &tint) {
                        Some(f) => {
                            painter.circle(c, r, f, if fill == Fill::Accent { Stroke::NONE } else { ink });
                        }
                        None => {
                            painter.circle_stroke(c, r, ink);
                        }
                    }
                }
                Shape::Line { points } => {
                    let pts: Vec<Pos2> = points.iter().map(|(x, y)| to_screen(*x, *y)).collect();
                    painter.add(egui::Shape::line(pts, ink));
                }
                Shape::Text { x, y, text, size, bold, light } => {
                    let px = (size as f32 * scale).clamp(7.0, 22.0);
                    let font = if bold { egui::FontId::new(px, egui::FontFamily::Proportional) } else { egui::FontId::proportional(px) };
                    let color = if light { Color32::WHITE } else { INK };
                    painter.text(to_screen(x, y), egui::Align2::CENTER_CENTER, text, font, color);
                }
            }
        }
        if let Some(item) = selected.and_then(|id| s.items.iter().find(|i| i.id == id)) {
            let r = egui::Rect::from_center_size(to_screen(item.x, item.y), egui::vec2(item.w as f32, item.d as f32) * scale);
            let r = if (item.rot / 90.0).round() as i64 % 2 != 0 { egui::Rect::from_center_size(r.center(), egui::vec2(r.height(), r.width())) } else { r };
            painter.rect_stroke(r.expand(4.0), 3.0, Stroke::new(2.0, theme::ACCENT), egui::StrokeKind::Outside);
        }

        // Picking: the topmost item under the pointer.
        let topmost = |pos: Pos2| -> Option<Id> {
            let (x, y) = to_stage(pos);
            let mut items: Vec<&StageItem> = s.items.iter().collect();
            items.sort_by_key(|i| i.symbol.layer());
            items.iter().rev().find(|i| i.hit(x, y)).map(|i| i.id)
        };
        if resp.clicked() || resp.drag_started() {
            resp.request_focus();
            let hit = resp.interact_pointer_pos().and_then(topmost);
            self.stage.selected = hit.map(|i| (plan, i));
            self.stage.drag = if resp.drag_started() { hit } else { None };
            if resp.drag_started() {
                doc.seal();
            }
        }
        if let Some(id) = self.stage.drag {
            if resp.dragged() {
                let d = resp.drag_delta() / scale;
                if d != egui::Vec2::ZERO {
                    doc.edit(&format!("stage:{plan}:{id}:move"), |p| {
                        if let Some(it) = plot_of(p, plan).and_then(|s| {
                            let (w, h) = (s.width, s.depth);
                            s.items.iter_mut().find(|i| i.id == id).map(|i| (i, w, h))
                        }) {
                            let (i, w, h) = it;
                            i.x = (i.x + d.x as f64).clamp(-1.0, w + 1.0);
                            i.y = (i.y + d.y as f64).clamp(-1.0, h + 1.0);
                        }
                    });
                }
            }
            if resp.drag_stopped() {
                let g = s.grid;
                doc.edit(&format!("stage:{plan}:{id}:move"), |p| {
                    if let Some(i) = plot_of(p, plan).and_then(|s| s.items.iter_mut().find(|i| i.id == id)) {
                        i.x = snap(i.x, g);
                        i.y = snap(i.y, g);
                    }
                });
                doc.seal();
                self.stage.drag = None;
            }
        }
        if let Some(pos) = resp.hover_pos() {
            if topmost(pos).is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        }

        // Keys while the stage has the focus.
        let mut remove = false;
        if let (true, Some(id)) = (resp.has_focus(), selected) {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    resp.id,
                    egui::EventFilter { tab: false, horizontal_arrows: true, vertical_arrows: true, escape: true },
                )
            });
            let (mut dx, mut dy, mut turn) = (0.0, 0.0, 0.0);
            ui.input(|i| {
                let step = if i.modifiers.shift { s.grid * 4.0 } else { s.grid.max(0.05) };
                if i.key_pressed(Key::ArrowLeft) {
                    dx -= step;
                }
                if i.key_pressed(Key::ArrowRight) {
                    dx += step;
                }
                if i.key_pressed(Key::ArrowUp) {
                    dy -= step;
                }
                if i.key_pressed(Key::ArrowDown) {
                    dy += step;
                }
                if i.key_pressed(Key::R) {
                    turn = if i.modifiers.shift { -45.0 } else { 90.0 };
                }
                remove = i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace);
                if i.key_pressed(Key::Escape) {
                    self.stage.selected = None;
                }
            });
            if dx != 0.0 || dy != 0.0 || turn != 0.0 {
                doc.edit(&format!("stage:{plan}:{id}:nudge"), |p| {
                    if let Some(i) = plot_of(p, plan).and_then(|s| s.items.iter_mut().find(|i| i.id == id)) {
                        i.x += dx;
                        i.y += dy;
                        i.rot = (i.rot + turn).rem_euclid(360.0);
                    }
                });
            }
        }
        ui.label(
            RichText::new("Click to select, drag to move (snaps to the grid). With the stage selected: arrows nudge (Shift: further), R turns, Delete removes.")
                .small()
                .color(theme::MUTED),
        );

        // The selected item.
        if let Some(item) = selected.and_then(|id| s.items.iter().find(|i| i.id == id)).cloned() {
            self.stage_properties(ui, plan, &project, &item, remove);
        }
        ui.add_space(8.0);
    }

    fn stage_add(&mut self, plan: Id, s: &StagePlot, sym: Symbol) {
        let Some(doc) = self.doc.as_mut() else { return };
        doc.seal();
        let (x, y) = (snap(s.width / 2.0, s.grid), snap(s.depth / 2.0, s.grid));
        let id = doc.edit(&format!("stage:{plan}:add"), |p| {
            let id = p.alloc_id();
            if let Some(s) = plot_of(p, plan) {
                // Not right on top of another new one.
                let mut item = StageItem::new(id, sym, x, y);
                while s.items.iter().any(|i| (i.x - item.x).abs() < 1e-6 && (i.y - item.y).abs() < 1e-6) {
                    item.x += s.grid.max(0.25);
                    item.y += s.grid.max(0.25);
                }
                s.items.push(item);
            }
            id
        });
        doc.seal();
        self.stage.selected = Some((plan, id));
    }

    fn stage_properties(&mut self, ui: &mut egui::Ui, plan: Id, project: &Project, item: &StageItem, remove_key: bool) {
        let Some(doc) = self.doc.as_mut() else { return };
        let mut error = None;
        let mut remove = remove_key;
        let mut duplicate = false;
        let mut turn = 0.0;
        egui::Frame::new().fill(theme::CARD).corner_radius(6.0).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(item.symbol.label()).strong());
                ui.label(RichText::new("Label").color(theme::MUTED));
                if let Some(v) = parsed_field(ui, ("stage_label", item.id), &item.label, "shown on the plot") {
                    doc.edit(&format!("stage:{plan}:{}:label", item.id), |p| {
                        if let Some(i) = plot_of(p, plan).and_then(|s| s.items.iter_mut().find(|i| i.id == item.id)) {
                            i.label = v.trim().to_string();
                        }
                    });
                }
                if item.symbol.takes_inputs() || !item.inputs.is_empty() {
                    ui.label(RichText::new("Inputs").color(theme::MUTED));
                    let shown = bp_core::stage::channels(project, item);
                    ui.scope(|ui| {
                        ui.set_max_width(110.0);
                        if let Some(v) = parsed_field(ui, ("stage_inputs", item.id), &shown, "ch, e.g. 3, 4") {
                            match parse_channels(project, &v) {
                                Ok(ids) => {
                                    doc.edit(&format!("stage:{plan}:{}:inputs", item.id), |p| {
                                        if let Some(i) = plot_of(p, plan).and_then(|s| s.items.iter_mut().find(|i| i.id == item.id)) {
                                            i.inputs = ids;
                                        }
                                    });
                                }
                                Err(e) => error = Some(format!("Inputs: {e}")),
                            }
                        }
                    });
                }
                ui.label(RichText::new(format!("at {} / {} m, {} deg", fmt(item.x), fmt(item.y), fmt(item.rot))).small().color(theme::MUTED));
                if ui.small_button("Turn left").clicked() {
                    turn = -90.0;
                }
                if ui.small_button("Turn right").clicked() {
                    turn = 90.0;
                }
                if ui.small_button("Duplicate").clicked() {
                    duplicate = true;
                }
                if ui.small_button("Remove").clicked() {
                    remove = true;
                }
            });
        });
        let id = item.id;
        if turn != 0.0 {
            doc.edit(&format!("stage:{plan}:{id}:nudge"), |p| {
                if let Some(i) = plot_of(p, plan).and_then(|s| s.items.iter_mut().find(|i| i.id == id)) {
                    i.rot = (i.rot + turn).rem_euclid(360.0);
                }
            });
        }
        if duplicate {
            doc.seal();
            let new = doc.edit(&format!("stage:{plan}:dup"), |p| {
                let new = p.alloc_id();
                let s = plot_of(p, plan)?;
                let mut copy = s.items.iter().find(|i| i.id == id)?.clone();
                copy.id = new;
                copy.x += s.grid.max(0.25) * 2.0;
                copy.inputs.clear();
                s.items.push(copy);
                Some(new)
            });
            doc.seal();
            if let Some(new) = new {
                self.stage.selected = Some((plan, new));
            }
        }
        if remove {
            doc.seal();
            doc.edit(&format!("stage:{plan}:remove"), |p| {
                if let Some(s) = plot_of(p, plan) {
                    s.items.retain(|i| i.id != id);
                }
            });
            doc.seal();
            self.stage.selected = None;
        }
        if let Some(e) = error {
            self.notify(e, true);
        }
    }
}

fn fmt(v: f64) -> String {
    bp_core::time::format_number(v)
}
