//! The table editor used for all lists: spreadsheet-like, fully usable from
//! the keyboard.
//!
//! - Arrows move, Shift+Up/Down select rows, Tab / Shift+Tab move sideways.
//! - Enter or F2 edits the cell, typing starts editing with that text,
//!   Backspace / Delete empties the cell. While editing: Enter keeps the text
//!   and goes down, Tab keeps it and goes right, Escape throws it away.
//! - Cmd/Ctrl+C copies the selected rows as TSV, Cmd/Ctrl+V pastes TSV from
//!   Excel / Numbers at the cursor (rows are added as needed).
//! - Cmd/Ctrl+Enter inserts a row below, Cmd/Ctrl+Backspace deletes the
//!   selected rows, Cmd/Ctrl+A selects all.
//! - Click selects, Shift+click selects a range, Cmd/Ctrl+click adds a row,
//!   double-click edits, drag the row number to reorder.
//!
//! The table only shows text and reports [`Action`]s; the caller changes the
//! project (with undo) and the table shows the result next frame.

use crate::theme;
use bp_core::model::COLORS;
use bp_core::sheet::{to_tsv, ColKind, Column};
use egui::{
    Align2, Color32, Event, EventFilter, FontId, Id, Key, Rect, Sense, Stroke, StrokeKind, Ui,
    UiBuilder,
};
use std::collections::BTreeSet;

pub const ROW_H: f32 = 28.0;
const HANDLE_W: f32 = 46.0;
const HEADER_H: f32 = 28.0;
const PAD: f32 = 6.0;

/// What the user did; the caller applies it.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Set { row: usize, col: usize, value: String },
    Paste { row: usize, col: usize, text: String },
    /// Insert an empty row at this index (and edit it).
    Insert { at: usize },
    Delete { rows: Vec<usize> },
    /// Move these rows before the row at `to`.
    Move { rows: Vec<usize>, to: usize },
}

#[derive(Clone, Debug)]
struct Editing {
    row: usize,
    col: usize,
    text: String,
    /// The text field still has to take the focus.
    start: bool,
}

#[derive(Clone, Debug, Default)]
struct Drag {
    rows: Vec<usize>,
    target: Option<usize>,
}

/// Cursor, selection and editing of one table (kept by the caller per list).
#[derive(Clone, Debug, Default)]
pub struct TableState {
    pub cursor: (usize, usize),
    pub selected: BTreeSet<usize>,
    anchor: usize,
    editing: Option<Editing>,
    drag: Option<Drag>,
    /// Scroll so the cursor row is visible.
    reveal: bool,
    offset: f32,
    view_h: f32,
    /// Give the table the keyboard focus next frame.
    focus: bool,
}

impl TableState {
    /// Select exactly these rows; the cursor goes to the first.
    pub fn select(&mut self, rows: &[usize]) {
        self.selected = rows.iter().copied().collect();
        if let Some(r) = rows.first() {
            self.cursor.0 = *r;
            self.anchor = *r;
        }
        self.reveal = true;
        self.focus = true;
    }

    /// Start editing a cell with its text empty (after inserting a row).
    pub fn edit_new(&mut self, row: usize, col: usize) {
        self.select(&[row]);
        self.cursor = (row, col);
        self.editing = Some(Editing {
            row,
            col,
            text: String::new(),
            start: true,
        });
    }

    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    fn rows_or_cursor(&self) -> Vec<usize> {
        if self.selected.is_empty() {
            vec![self.cursor.0]
        } else {
            self.selected.iter().copied().collect()
        }
    }

    fn move_to(&mut self, row: usize, col: usize, extend: bool) {
        self.cursor = (row, col);
        if extend {
            let (a, b) = (self.anchor.min(row), self.anchor.max(row));
            self.selected = (a..=b).collect();
        } else {
            self.selected = [row].into_iter().collect();
            self.anchor = row;
        }
        self.reveal = true;
    }

    fn clamp(&mut self, rows: usize, cols: usize) {
        if rows == 0 {
            self.cursor = (0, self.cursor.1.min(cols.saturating_sub(1)));
            self.selected.clear();
            self.editing = None;
            return;
        }
        self.cursor.0 = self.cursor.0.min(rows - 1);
        self.cursor.1 = self.cursor.1.min(cols.saturating_sub(1));
        self.selected.retain(|r| *r < rows);
        if self.editing.as_ref().is_some_and(|e| e.row >= rows) {
            self.editing = None;
        }
    }
}

/// What to show.
pub struct TableView<'a> {
    pub id: Id,
    pub columns: &'a [Column],
    pub rows: usize,
    pub text: &'a dyn Fn(usize, usize) -> String,
    /// Colour of a row (colour coding), if any.
    pub color: &'a dyn Fn(usize) -> Option<Color32>,
    /// Names offered in person columns.
    pub people: &'a [String],
    /// Shown when there are no rows.
    pub empty_hint: &'a str,
}

impl TableView<'_> {
    fn choices(&self, col: usize) -> Option<Vec<String>> {
        match &self.columns.get(col)?.kind {
            ColKind::Choice(c) => Some(c.iter().map(|s| s.to_string()).collect()),
            ColKind::Color => Some(COLORS.iter().map(|s| s.to_string()).collect()),
            ColKind::Person => Some(
                std::iter::once(String::new())
                    .chain(self.people.iter().cloned())
                    .collect(),
            ),
            _ => None,
        }
    }

    fn copy(&self, rows: &[usize]) -> String {
        let grid: Vec<Vec<String>> = rows
            .iter()
            .filter(|r| **r < self.rows)
            .map(|r| (0..self.columns.len()).map(|c| (self.text)(*r, c)).collect())
            .collect();
        to_tsv(&grid)
    }
}

/// Draw the table and handle input.
pub fn show(ui: &mut Ui, state: &mut TableState, view: &TableView) -> Vec<Action> {
    let mut actions = Vec::new();
    let cols = view.columns.len();
    state.clamp(view.rows, cols);
    let id = view.id;
    let edit_id = id.with("edit");
    let widths: Vec<f32> = view.columns.iter().map(|c| c.width).collect();
    let total_w = HANDLE_W + widths.iter().sum::<f32>();

    if state.focus {
        ui.memory_mut(|m| m.request_focus(id));
        state.focus = false;
    }
    let focused = ui.memory(|m| m.has_focus(id));
    if focused {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            )
        });
        if state.editing.is_none() {
            keyboard(ui, state, view, &mut actions);
        }
    }

    // Keyboard moves scroll the cursor row into view.
    let mut set_offset = None;
    if state.reveal && state.view_h > 0.0 {
        let top = state.cursor.0 as f32 * ROW_H;
        if top < state.offset {
            set_offset = Some(top);
        } else if top + ROW_H > state.offset + state.view_h {
            set_offset = Some(top + ROW_H - state.view_h);
        }
    }
    state.reveal = false;

    egui::ScrollArea::horizontal()
        .id_salt(id.with("h"))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            // The whole area takes the focus when clicked (cells sit on top).
            let area = Rect::from_min_size(
                ui.cursor().min,
                egui::vec2(total_w.max(ui.available_width()), ui.available_height()),
            );
            let bg = ui.interact(area, id, Sense::click());
            if bg.clicked() {
                ui.memory_mut(|m| m.request_focus(id));
            }
            header(ui, view, &widths, total_w);
            if view.rows == 0 {
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.add_space(PAD + 4.0);
                    let hint = egui::Label::new(egui::RichText::new(view.empty_hint).color(theme::MUTED))
                        .sense(Sense::click());
                    if ui.add(hint).clicked() {
                        ui.memory_mut(|m| m.request_focus(id));
                    }
                });
                return;
            }
            let mut area = egui::ScrollArea::vertical()
                .id_salt(id.with("v"))
                .auto_shrink([false, false]);
            if let Some(o) = set_offset {
                area = area.vertical_scroll_offset(o);
            }
            let out = area.show_rows(ui, ROW_H, view.rows, |ui, range| {
                for row in range {
                    draw_row(ui, state, view, row, &widths, total_w, focused, &mut actions);
                }
            });
            state.offset = out.state.clamped_offset().y;
            state.view_h = out.inner_rect.height();
        });

    // Finish a drag (also when released outside the table).
    if state.drag.is_some() && ui.input(|i| i.pointer.any_released()) {
        if let Some(d) = state.drag.take() {
            if let Some(to) = d.target {
                actions.push(Action::Move { rows: d.rows, to });
            }
        }
    } else if state.drag.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }

    // Editing: commit / cancel when the text field gives up the focus.
    if let Some(e) = &state.editing {
        if !e.start && !ui.memory(|m| m.has_focus(edit_id)) {
            let e = e.clone();
            state.editing = None;
            let (esc, enter, tab, shift) = ui.input(|i| {
                (
                    i.key_pressed(Key::Escape),
                    i.key_pressed(Key::Enter),
                    i.key_pressed(Key::Tab),
                    i.modifiers.shift,
                )
            });
            if !esc {
                actions.push(Action::Set {
                    row: e.row,
                    col: e.col,
                    value: e.text,
                });
            }
            if esc || enter || tab {
                state.focus = true;
            }
            if enter && e.row + 1 < view.rows {
                state.move_to(e.row + 1, e.col, false);
            } else if tab {
                let (r, c) = step(e.row, e.col, view.rows, cols, !shift);
                state.move_to(r, c, false);
            }
        }
    }
    actions
}

/// The next cell sideways (wrapping to the next / previous row).
fn step(row: usize, col: usize, rows: usize, cols: usize, forward: bool) -> (usize, usize) {
    if forward {
        if col + 1 < cols {
            (row, col + 1)
        } else if row + 1 < rows {
            (row + 1, 0)
        } else {
            (row, col)
        }
    } else if col > 0 {
        (row, col - 1)
    } else if row > 0 {
        (row - 1, cols.saturating_sub(1))
    } else {
        (row, col)
    }
}

fn keyboard(ui: &mut Ui, state: &mut TableState, view: &TableView, actions: &mut Vec<Action>) {
    let cols = view.columns.len();
    let rows = view.rows;
    let events = ui.input(|i| i.events.clone());
    for ev in events {
        match ev {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                let (r, c) = state.cursor;
                let cmd = modifiers.command;
                match key {
                    Key::ArrowUp if r > 0 => state.move_to(r - 1, c, modifiers.shift),
                    Key::ArrowDown if r + 1 < rows => state.move_to(r + 1, c, modifiers.shift),
                    Key::ArrowLeft if c > 0 => state.move_to(r, c - 1, false),
                    Key::ArrowRight if c + 1 < cols => state.move_to(r, c + 1, false),
                    Key::Home => state.move_to(r, 0, false),
                    Key::End => state.move_to(r, cols.saturating_sub(1), false),
                    Key::Tab if rows > 0 => {
                        let (r, c) = step(r, c, rows, cols, !modifiers.shift);
                        state.move_to(r, c, false);
                    }
                    Key::Enter if cmd => actions.push(Action::Insert {
                        at: if rows == 0 { 0 } else { r + 1 },
                    }),
                    Key::Enter | Key::F2 if rows > 0 => {
                        state.editing = Some(Editing {
                            row: r,
                            col: c,
                            text: (view.text)(r, c),
                            start: true,
                        });
                    }
                    Key::Backspace | Key::Delete if cmd && rows > 0 => {
                        actions.push(Action::Delete {
                            rows: state.rows_or_cursor(),
                        });
                    }
                    Key::Backspace | Key::Delete if rows > 0 => actions.push(Action::Set {
                        row: r,
                        col: c,
                        value: String::new(),
                    }),
                    Key::A if cmd => {
                        state.selected = (0..rows).collect();
                    }
                    Key::Escape => {
                        state.selected = [r].into_iter().collect();
                        state.anchor = r;
                    }
                    _ => {}
                }
            }
            Event::Text(t) if rows > 0 && state.editing.is_none() => {
                let (r, c) = state.cursor;
                state.editing = Some(Editing {
                    row: r,
                    col: c,
                    text: t,
                    start: true,
                });
            }
            Event::Copy | Event::Cut if rows > 0 => {
                ui.ctx().copy_text(view.copy(&state.rows_or_cursor()));
            }
            Event::Paste(text) => {
                let (r, c) = state.cursor;
                actions.push(Action::Paste {
                    row: if rows == 0 { 0 } else { r },
                    col: if rows == 0 { 0 } else { c },
                    text,
                });
            }
            _ => {}
        }
    }
}

fn header(ui: &mut Ui, view: &TableView, widths: &[f32], total_w: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(total_w, HEADER_H), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, theme::CARD);
    p.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, theme::LINE));
    let mut x = rect.left() + HANDLE_W;
    p.text(
        egui::pos2(rect.left() + HANDLE_W - PAD, rect.center().y),
        Align2::RIGHT_CENTER,
        "#",
        FontId::proportional(12.0),
        theme::MUTED,
    );
    for (c, w) in view.columns.iter().zip(widths) {
        let r = Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(*w, HEADER_H));
        p.with_clip_rect(r.shrink(2.0)).text(
            egui::pos2(r.left() + PAD, r.center().y),
            Align2::LEFT_CENTER,
            c.title,
            FontId::proportional(12.5),
            theme::MUTED,
        );
        p.vline(r.right() - 0.5, r.y_range(), Stroke::new(1.0, theme::LINE));
        x += w;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_row(
    ui: &mut Ui,
    state: &mut TableState,
    view: &TableView,
    row: usize,
    widths: &[f32],
    total_w: f32,
    focused: bool,
    actions: &mut Vec<Action>,
) {
    let id = view.id;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(total_w, ROW_H), Sense::hover());
    let selected = state.selected.contains(&row);
    let color = (view.color)(row);
    {
        let p = ui.painter();
        let fill = if selected {
            theme::ACCENT_BG
        } else if let Some(c) = color {
            c.gamma_multiply(0.10)
        } else if row % 2 == 1 {
            Color32::from_rgb(20, 22, 28)
        } else {
            theme::PANEL
        };
        p.rect_filled(rect, 0.0, fill);
        p.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, Color32::from_rgb(30, 33, 41)));
        if let Some(c) = color {
            p.rect_filled(
                Rect::from_min_size(rect.min, egui::vec2(4.0, ROW_H)),
                0.0,
                c,
            );
        }
    }

    // Row number: select, drag to reorder.
    let handle = Rect::from_min_size(rect.min, egui::vec2(HANDLE_W, ROW_H));
    let hr = ui.interact(handle, id.with(("handle", row)), Sense::click_and_drag());
    ui.painter().text(
        egui::pos2(handle.right() - PAD, handle.center().y),
        Align2::RIGHT_CENTER,
        (row + 1).to_string(),
        FontId::monospace(12.0),
        if hr.hovered() { theme::TEXT } else { theme::MUTED },
    );
    if hr.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    if hr.clicked() {
        click_select(ui, state, id, row, state.cursor.1);
    }
    if hr.drag_started() {
        if !state.selected.contains(&row) {
            state.move_to(row, state.cursor.1, false);
        }
        state.drag = Some(Drag {
            rows: state.selected.iter().copied().collect(),
            target: None,
        });
        ui.memory_mut(|m| m.request_focus(id));
    }
    if let (Some(d), Some(pos)) = (state.drag.as_mut(), ui.ctx().pointer_interact_pos()) {
        if pos.y >= rect.top() && pos.y < rect.bottom() {
            let to = if pos.y < rect.center().y { row } else { row + 1 };
            d.target = Some(to);
        }
        if let Some(to) = d.target {
            let y = if to == row {
                Some(rect.top())
            } else if to == row + 1 {
                Some(rect.bottom())
            } else {
                None
            };
            if let Some(y) = y {
                ui.painter()
                    .hline(rect.x_range(), y, Stroke::new(2.5, theme::ACCENT));
            }
        }
    }

    let mut x = rect.left() + HANDLE_W;
    for (col, w) in widths.iter().enumerate() {
        let cell = Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(*w, ROW_H));
        x += w;
        let is_cursor = state.cursor == (row, col);
        let editing_here = state
            .editing
            .as_ref()
            .is_some_and(|e| e.row == row && e.col == col);
        if editing_here {
            edit_cell(ui, state, view, cell);
            continue;
        }
        let resp = ui.interact(cell, id.with(("cell", row, col)), Sense::click());
        let text = (view.text)(row, col);
        let p = ui.painter();
        if view.columns[col].kind == ColKind::Color {
            if let Some(c) = theme::color(&text) {
                p.circle_filled(egui::pos2(cell.left() + PAD + 6.0, cell.center().y), 6.0, c);
            }
            p.with_clip_rect(cell.shrink(1.0)).text(
                egui::pos2(cell.left() + PAD + 18.0, cell.center().y),
                Align2::LEFT_CENTER,
                &text,
                FontId::proportional(13.5),
                theme::TEXT,
            );
        } else {
            p.with_clip_rect(cell.shrink(1.0)).text(
                egui::pos2(cell.left() + PAD, cell.center().y),
                Align2::LEFT_CENTER,
                &text,
                FontId::proportional(13.5),
                theme::TEXT,
            );
        }
        p.vline(
            cell.right() - 0.5,
            cell.y_range(),
            Stroke::new(1.0, Color32::from_rgb(30, 33, 41)),
        );
        if is_cursor {
            p.rect_stroke(
                cell.shrink(1.0),
                2.0,
                Stroke::new(1.5, if focused { theme::ACCENT } else { theme::MUTED }),
                StrokeKind::Inside,
            );
        }
        if resp.clicked() {
            click_select(ui, state, id, row, col);
        }
        if resp.double_clicked() {
            state.editing = Some(Editing {
                row,
                col,
                text,
                start: true,
            });
        }
        // Choice columns: a menu of the possible values (also on right-click).
        if let Some(choices) = view.choices(col) {
            let pick = |ui: &mut Ui, actions: &mut Vec<Action>| {
                for c in &choices {
                    let label = if c.is_empty() { "(none)" } else { c.as_str() };
                    if ui.button(label).clicked() {
                        actions.push(Action::Set {
                            row,
                            col,
                            value: c.clone(),
                        });
                        ui.close();
                    }
                }
            };
            if is_cursor || resp.hovered() {
                let b = Rect::from_min_size(
                    egui::pos2(cell.right() - 22.0, cell.top() + 4.0),
                    egui::vec2(18.0, ROW_H - 8.0),
                );
                ui.scope_builder(UiBuilder::new().max_rect(b), |ui| {
                    let r = ui.menu_button("  ", |ui| pick(ui, actions)).response;
                    // A small down arrow (the default fonts have no glyph for it).
                    let c = r.rect.center();
                    ui.painter().add(egui::Shape::convex_polygon(
                        vec![
                            egui::pos2(c.x - 4.0, c.y - 2.0),
                            egui::pos2(c.x + 4.0, c.y - 2.0),
                            egui::pos2(c.x, c.y + 3.0),
                        ],
                        theme::MUTED,
                        egui::Stroke::NONE,
                    ));
                });
            }
            resp.context_menu(|ui| {
                pick(ui, actions);
                ui.separator();
                row_menu(ui, state, row, actions);
            });
        } else {
            resp.context_menu(|ui| row_menu(ui, state, row, actions));
        }
    }
}

fn row_menu(ui: &mut Ui, state: &mut TableState, row: usize, actions: &mut Vec<Action>) {
    if ui.button("Insert row above").clicked() {
        actions.push(Action::Insert { at: row });
        ui.close();
    }
    if ui.button("Insert row below").clicked() {
        actions.push(Action::Insert { at: row + 1 });
        ui.close();
    }
    let rows = if state.selected.contains(&row) {
        state.selected.iter().copied().collect()
    } else {
        vec![row]
    };
    let label = if rows.len() > 1 {
        format!("Delete {} rows", rows.len())
    } else {
        "Delete row".to_string()
    };
    if ui.button(label).clicked() {
        actions.push(Action::Delete { rows });
        ui.close();
    }
}

fn click_select(ui: &mut Ui, state: &mut TableState, id: Id, row: usize, col: usize) {
    let m = ui.input(|i| i.modifiers);
    state.cursor = (row, col);
    if m.shift {
        let (a, b) = (state.anchor.min(row), state.anchor.max(row));
        state.selected = (a..=b).collect();
    } else if m.command {
        if !state.selected.remove(&row) {
            state.selected.insert(row);
        }
        state.anchor = row;
    } else {
        state.selected = [row].into_iter().collect();
        state.anchor = row;
    }
    ui.memory_mut(|m| m.request_focus(id));
}

fn edit_cell(ui: &mut Ui, state: &mut TableState, view: &TableView, cell: Rect) {
    let edit_id = view.id.with("edit");
    let Some(e) = state.editing.as_mut() else {
        return;
    };
    let te = egui::TextEdit::singleline(&mut e.text)
        .id(edit_id)
        .font(FontId::proportional(13.5))
        .margin(egui::Margin::symmetric(5, 4))
        .desired_width(cell.width() - 2.0);
    let resp = ui.put(cell.shrink(1.0), te);
    if e.start {
        resp.request_focus();
        // Cursor at the end of the text.
        let mut ts = egui::text_edit::TextEditState::load(ui.ctx(), edit_id).unwrap_or_default();
        let end = egui::text::CCursor::new(e.text.chars().count());
        ts.cursor
            .set_char_range(Some(egui::text::CCursorRange::one(end)));
        ts.store(ui.ctx(), edit_id);
        e.start = false;
    }
    ui.painter().rect_stroke(
        cell.shrink(1.0),
        2.0,
        Stroke::new(1.5, theme::ACCENT),
        StrokeKind::Inside,
    );
}
