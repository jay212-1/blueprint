//! Import into any table (with the columns matched by the person) and
//! export of a table as CSV.

use crate::app::App;
use crate::theme;
use bp_core::grid::{ColKind, Grid};
use bp_core::import::{auto_map, has_header, import, read_file, to_csv, Table};
use egui::RichText;
use std::path::{Path, PathBuf};

/// An import waiting for its columns to be checked (shown above the table).
pub struct ImportState {
    /// The table it goes into.
    pub key: String,
    pub file: String,
    pub tables: Vec<Table>,
    pub sheet: usize,
    /// The first row holds titles (not imported).
    pub header: bool,
    /// The table column for each source column.
    pub map: Vec<Option<usize>>,
}

impl ImportState {
    fn table(&self) -> Option<&Table> {
        self.tables.get(self.sheet)
    }

    fn rows(&self) -> &[Vec<String>] {
        match self.table() {
            Some(t) if self.header => t.rows.get(1..).unwrap_or(&[]),
            Some(t) => &t.rows,
            None => &[],
        }
    }

    /// Match the columns again (another sheet, header on or off).
    fn remap(&mut self, columns: &[bp_core::grid::Column]) {
        let Some(t) = self.table() else { return };
        let width = t.width();
        let mut map = match (self.header, t.rows.first()) {
            (true, Some(first)) => auto_map(first, columns),
            _ => Vec::new(),
        };
        map.resize(width, None);
        // Without titles: the columns in order, as a paste would.
        if !self.header {
            let mut targets = (0..columns.len()).filter(|c| columns[*c].kind != ColKind::Fixed);
            for m in map.iter_mut() {
                *m = targets.next();
            }
        }
        self.map = map;
    }
}

impl App {
    pub(crate) fn pick_import(&self) -> Option<PathBuf> {
        if !self.dialogs {
            return None;
        }
        let mut d = rfd::FileDialog::new()
            .set_title("Import rows")
            .add_filter("Tables", &["csv", "tsv", "txt", "xlsx", "xlsm", "xls", "ods"]);
        if let Some(dir) = &self.settings.last_dir {
            d = d.set_directory(dir);
        }
        d.pick_file()
    }

    pub(crate) fn pick_csv(&self, grid: &dyn Grid) -> Option<PathBuf> {
        if !self.dialogs {
            return None;
        }
        let name = self.doc.as_ref().map(|d| d.title()).unwrap_or_else(|| "Untitled".into());
        let what = grid.key().replace([':', '/'], " ");
        let mut d = rfd::FileDialog::new()
            .set_title("Export CSV")
            .add_filter("CSV", &["csv"])
            .set_file_name(format!("{name} - {}.csv", what.trim()));
        if let Some(dir) = &self.settings.last_dir {
            d = d.set_directory(dir);
        }
        d.save_file().map(|p| if p.extension().is_none() { p.with_extension("csv") } else { p })
    }

    /// Read a file for the table and show the column matching. False when
    /// the file could not be read (the message says why).
    pub fn start_import(&mut self, grid: &dyn Grid, path: &Path) -> bool {
        let Some(doc) = self.doc.as_ref() else { return false };
        match read_file(path) {
            Ok(tables) => {
                let columns = grid.columns(&doc.project);
                let header = tables.first().and_then(|t| t.rows.first()).is_some_and(|r| has_header(r, &columns));
                let mut st = ImportState {
                    key: grid.key(),
                    file: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                    tables,
                    sheet: 0,
                    header,
                    map: Vec::new(),
                };
                st.remap(&columns);
                self.import = Some(st);
                true
            }
            Err(e) => {
                self.notify(format!("Could not import: {e}"), true);
                false
            }
        }
    }

    pub(crate) fn import_panel(&mut self, ui: &mut egui::Ui, grid: &dyn Grid) {
        let (Some(doc), Some(st)) = (self.doc.as_ref(), self.import.as_mut()) else { return };
        let columns = grid.columns(&doc.project);
        let mut done = None;
        egui::Frame::new()
            .fill(theme::CARD)
            .stroke(egui::Stroke::new(1.0, theme::ACCENT))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Import from {}", st.file)).strong());
                    if st.tables.len() > 1 {
                        let before = st.sheet;
                        egui::ComboBox::from_id_salt("import_sheet")
                            .selected_text(st.table().map(|t| t.name.clone()).unwrap_or_default())
                            .show_ui(ui, |ui| {
                                for (i, t) in st.tables.iter().enumerate() {
                                    ui.selectable_value(&mut st.sheet, i, t.name.as_str());
                                }
                            });
                        if st.sheet != before {
                            st.remap(&columns);
                        }
                    }
                    if ui.checkbox(&mut st.header, "First row has the titles").changed() {
                        st.remap(&columns);
                    }
                });
                ui.label(
                    RichText::new("Choose the column each one goes into (\"skip\" leaves it out). The rows are added at the end; Cmd/Ctrl+Z takes the whole import back.")
                        .small()
                        .color(theme::MUTED),
                );
                ui.add_space(4.0);
                let rows = st.rows().to_vec();
                let titles: Vec<String> = match (st.header, st.table().and_then(|t| t.rows.first())) {
                    (true, Some(first)) => first.clone(),
                    _ => Vec::new(),
                };
                egui::ScrollArea::horizontal().id_salt("import_cols").show(ui, |ui| {
                    egui::Grid::new("import_map").striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                        for (i, m) in st.map.iter_mut().enumerate() {
                            let title = titles.get(i).filter(|t| !t.is_empty()).cloned().unwrap_or_else(|| format!("Column {}", i + 1));
                            ui.vertical(|ui| {
                                ui.label(RichText::new(title).strong());
                                let shown = m.and_then(|c| columns.get(c)).map_or("skip".to_string(), |c| c.title.clone());
                                egui::ComboBox::from_id_salt(("import_col", i)).width(130.0).selected_text(shown).show_ui(ui, |ui| {
                                    ui.selectable_value(m, None, "skip");
                                    for (c, col) in columns.iter().enumerate() {
                                        if col.kind != ColKind::Fixed {
                                            ui.selectable_value(m, Some(c), col.title.as_str());
                                        }
                                    }
                                });
                                for r in rows.iter().take(3) {
                                    let v = r.get(i).cloned().unwrap_or_default();
                                    let v: String = if v.chars().count() > 22 { v.chars().take(21).chain(std::iter::once('.')).collect() } else { v };
                                    ui.label(RichText::new(v).small().color(theme::MUTED));
                                }
                            });
                        }
                        ui.end_row();
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let n = rows.len();
                    let any = st.map.iter().any(Option::is_some);
                    if ui
                        .add_enabled(any && n > 0, egui::Button::new(RichText::new(format!("Import {n} row{}", if n == 1 { "" } else { "s" })).strong()))
                        .clicked()
                    {
                        done = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        done = Some(false);
                    }
                    if !any {
                        ui.label(RichText::new("Choose at least one column.").color(theme::WARN));
                    }
                });
            });
        ui.add_space(8.0);
        match done {
            Some(true) => self.finish_import(grid),
            Some(false) => self.import = None,
            None => {}
        }
    }

    /// Add the rows of the import shown for this table.
    pub fn finish_import(&mut self, grid: &dyn Grid) {
        let Some(st) = self.import.take() else { return };
        if st.key != grid.key() {
            return;
        }
        let Some(doc) = self.doc.as_mut() else { return };
        let first = if st.header { 2 } else { 1 };
        let rows = st.rows().to_vec();
        doc.seal();
        let before = grid.len(&doc.project);
        let report = doc.edit("import", |p| import(grid, p, &rows, &st.map, first));
        doc.seal();
        let after = grid.len(&doc.project);
        let state = self.tables.entry(st.key.clone()).or_default();
        state.select(&(before..after).collect::<Vec<_>>());
        let added = format!("Imported {} row{}.", report.rows, if report.rows == 1 { "" } else { "s" });
        if report.problems.is_empty() {
            self.notify(added, false);
        } else {
            let n = report.problems.len();
            let mut list = report.problems.iter().take(15).cloned().collect::<Vec<_>>().join("\n");
            if n > 15 {
                list.push_str(&format!("\n… and {} more", n - 15));
            }
            self.message(format!("{added} These values were not taken (fix them in the table):\n\n{list}"));
        }
    }

    /// Write the table as CSV. True when written.
    pub fn export_csv(&mut self, grid: &dyn Grid, path: &Path) -> bool {
        let Some(doc) = self.doc.as_ref() else { return false };
        // A BOM so Excel reads umlauts right.
        let mut bytes = b"\xef\xbb\xbf".to_vec();
        bytes.extend(to_csv(grid, &doc.project).into_bytes());
        match bp_core::project::atomic_write(path, &bytes) {
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
}
