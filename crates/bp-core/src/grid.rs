//! Anything shown in the table editor: rows and columns of text over the
//! project. The master data lists (`sheet.rs`) and the views of the plans
//! (`mic_grid.rs` …) implement [`Grid`]; copy and paste (TSV, as Excel and
//! Numbers use it) work the same for all of them.

use crate::model::Project;

/// What a column holds (decides the editor and the parsing).
#[derive(Clone, Debug, PartialEq)]
pub enum ColKind {
    Text,
    /// A number (e.g. a frequency in MHz).
    Number,
    /// A duration typed as "m:ss".
    Duration,
    /// One of [`crate::model::COLORS`].
    Color,
    /// One of a fixed set of words (the editor offers them in a menu).
    Choice(Vec<String>),
    /// A person from the cast, by name.
    Person,
    /// Shown, not edited (worked out from other data).
    Fixed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub title: String,
    pub kind: ColKind,
    /// Suggested width in points.
    pub width: f32,
}

pub fn col(title: impl Into<String>, kind: ColKind, width: f32) -> Column {
    Column {
        title: title.into(),
        kind,
        width,
    }
}

/// How a single cell is marked (on top of the row colour).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CellMark {
    /// Colour name of the cell ([`crate::model::COLORS`]).
    pub color: String,
    /// Draw it as a problem.
    pub warn: bool,
    /// Shown when the pointer rests on the cell.
    pub hint: String,
}

pub trait Grid {
    /// Names the table (undo steps and the editor's state are kept per key).
    fn key(&self) -> String;
    fn columns(&self, p: &Project) -> Vec<Column>;
    fn len(&self, p: &Project) -> usize;
    /// The text shown in (and edited from) a cell.
    fn text(&self, p: &Project, row: usize, col: usize) -> String;
    /// Set a cell from text. On an error nothing changes.
    fn set(&self, p: &mut Project, row: usize, col: usize, value: &str) -> Result<(), String>;

    /// The row's colour name ("" = none).
    fn color(&self, _p: &Project, _row: usize) -> String {
        String::new()
    }
    fn mark(&self, _p: &Project, _row: usize, _col: usize) -> Option<CellMark> {
        None
    }
    /// Rows can be added, removed and moved.
    fn editable_rows(&self) -> bool {
        false
    }
    /// Insert an empty row at `at`; returns its index.
    fn insert(&self, _p: &mut Project, at: usize) -> usize {
        at
    }
    fn remove(&self, _p: &mut Project, _rows: &[usize]) {}
    /// Move rows before the row at `to`; returns their new indices.
    fn move_rows(&self, _p: &mut Project, rows: &[usize], _to: usize) -> Vec<usize> {
        rows.to_vec()
    }
}

/// The given rows as TSV (all columns).
pub fn copy(g: &dyn Grid, p: &Project, rows: &[usize]) -> String {
    let cols = g.columns(p).len();
    let grid: Vec<Vec<String>> = rows
        .iter()
        .map(|r| (0..cols).map(|c| g.text(p, *r, c)).collect())
        .collect();
    to_tsv(&grid)
}

/// Paste a TSV grid with its top left cell at (`row`, `column`); rows are
/// added at the end as needed (where the grid allows it). Fixed columns are
/// skipped. Returns the rows touched and one message per cell that could not
/// be read (those cells are left as they were).
pub fn paste(
    g: &dyn Grid,
    p: &mut Project,
    row: usize,
    column: usize,
    tsv: &str,
) -> (Vec<usize>, Vec<String>) {
    let grid = parse_tsv(tsv);
    let cols = g.columns(p);
    let mut touched = Vec::new();
    let mut errors = Vec::new();
    for (i, cells) in grid.iter().enumerate() {
        let r = row + i;
        if g.len(p) <= r {
            if !g.editable_rows() {
                break;
            }
            while g.len(p) <= r {
                let n = g.len(p);
                g.insert(p, n);
            }
        }
        touched.push(r);
        for (j, cell) in cells.iter().enumerate() {
            let c = column + j;
            let Some(column) = cols.get(c) else { break };
            if column.kind == ColKind::Fixed {
                continue;
            }
            if let Err(e) = g.set(p, r, c, cell) {
                errors.push(format!("row {}, {}: {e}", r + 1, column.title));
            }
        }
    }
    (touched, errors)
}

/// Cells to TSV; cells with tabs, line breaks or quotes are quoted (as Excel does).
pub fn to_tsv(grid: &[Vec<String>]) -> String {
    let cell = |s: &String| {
        if s.contains(['\t', '\n', '\r', '"']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.clone()
        }
    };
    grid.iter()
        .map(|row| row.iter().map(cell).collect::<Vec<_>>().join("\t"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// TSV to cells (Excel quoting understood; a trailing line break ignored).
pub fn parse_tsv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    let mut at_start = true;
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cell.push('"');
                } else {
                    quoted = false;
                }
            } else {
                cell.push(c);
            }
            continue;
        }
        match c {
            '"' if at_start => {
                quoted = true;
                at_start = false;
            }
            '\t' => {
                row.push(std::mem::take(&mut cell));
                at_start = true;
            }
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
                at_start = true;
            }
            _ => {
                cell.push(c);
                at_start = false;
            }
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}
