//! Importing rows from other programs into any table, and exporting a
//! table as CSV. Files are CSV / TSV / text (UTF-8 or Windows-1252, the
//! delimiter is guessed) or spreadsheets (XLSX, XLS, ODS). The source
//! columns are matched to the table's columns by their titles; the person
//! can change that before importing. Every cell goes through the table's
//! own parsing, so an import behaves like typing the values in.

use crate::grid::{Column, Grid};
use crate::model::Project;
use std::path::Path;

/// One sheet of a file: rows of cells.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub name: String,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    /// The number of columns (the longest row).
    pub fn width(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }
}

/// Read a file to its sheets (one for CSV and text).
pub fn read_file(path: &Path) -> Result<Vec<Table>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    read_bytes(&name, &bytes)
}

/// Read file contents; `name` gives the kind by its extension.
pub fn read_bytes(name: &str, bytes: &[u8]) -> Result<Vec<Table>, String> {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    let spreadsheet = matches!(ext.as_str(), "xlsx" | "xlsm" | "xlsb" | "xls" | "ods")
        || bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(&[0xd0, 0xcf, 0x11, 0xe0]);
    let mut tables = if spreadsheet {
        read_spreadsheet(bytes)?
    } else {
        let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
        vec![Table { name: stem.to_string(), rows: parse_text(&decode(bytes)) }]
    };
    for t in &mut tables {
        trim(&mut t.rows);
    }
    tables.retain(|t| !t.rows.is_empty());
    if tables.is_empty() {
        return Err("the file has no rows".into());
    }
    Ok(tables)
}

fn read_spreadsheet(bytes: &[u8]) -> Result<Vec<Table>, String> {
    use calamine::Reader;
    let mut book = calamine::open_workbook_auto_from_rs(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("not a spreadsheet Blueprint can read ({e})"))?;
    Ok(book
        .worksheets()
        .into_iter()
        .map(|(name, range)| Table { name, rows: range.rows().map(|r| r.iter().map(cell_text).collect()).collect() })
        .collect())
}

fn cell_text(d: &calamine::Data) -> String {
    use calamine::Data;
    match d {
        Data::Int(i) => i.to_string(),
        Data::Float(f) => crate::time::format_number(*f),
        Data::String(s) => s.trim().to_string(),
        Data::Bool(b) => if *b { "yes".into() } else { String::new() },
        Data::DateTime(t) => {
            let v = t.as_f64();
            if t.is_duration() {
                crate::time::format_duration(v * 86400.0)
            } else if v < 1.0 {
                crate::time::format_clock(v * 86400.0)
            } else {
                excel_date(v)
            }
        }
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(_) | Data::Empty => String::new(),
    }
}

/// An Excel day number as "2027-08-14" (with " 20:00" when it has a time).
fn excel_date(v: f64) -> String {
    // Day 25569 is 1970-01-01 (Excel counts a 29 Feb 1900 that never was).
    let days = v.floor() as i64 - 25569;
    let (y, m, d) = civil(days);
    let secs = (v.fract() * 86400.0).round();
    if secs >= 60.0 {
        format!("{y:04}-{m:02}-{d:02} {}", crate::time::format_clock(secs))
    } else {
        format!("{y:04}-{m:02}-{d:02}")
    }
}

/// Days since 1970-01-01 to (year, month, day).
fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// UTF-8 (without a BOM), else Windows-1252 as older Excel writes it.
pub fn decode(bytes: &[u8]) -> String {
    let b = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    if let Ok(s) = std::str::from_utf8(b) {
        return s.to_string();
    }
    // UTF-16 with a BOM (Excel's "Unicode text").
    if b.len() >= 2 && (b.starts_with(&[0xff, 0xfe]) || b.starts_with(&[0xfe, 0xff])) {
        let le = b[0] == 0xff;
        let units: Vec<u16> = b[2..]
            .chunks_exact(2)
            .map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) })
            .collect();
        return String::from_utf16_lossy(&units);
    }
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•',
        '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
    ];
    b.iter()
        .map(|&c| match c {
            0x80..=0x9f => HIGH[(c - 0x80) as usize],
            _ => c as char,
        })
        .collect()
}

/// Split text into rows and cells: tab, semicolon or comma, whichever the
/// first lines use most; quotes as in CSV.
pub fn parse_text(text: &str) -> Vec<Vec<String>> {
    let head: Vec<&str> = text.lines().take(10).collect();
    let count = |d: char| -> usize { head.iter().map(|l| l.matches(d).count()).sum() };
    let delim = ['\t', ';', ','].into_iter().max_by_key(|d| (count(*d), *d == '\t')).filter(|d| count(*d) > 0).unwrap_or('\t');
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
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
        } else if c == '"' && cell.trim().is_empty() {
            cell.clear();
            quoted = true;
        } else if c == delim {
            row.push(std::mem::take(&mut cell).trim().to_string());
        } else if c == '\n' || c == '\r' {
            if c == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            row.push(std::mem::take(&mut cell).trim().to_string());
            rows.push(std::mem::take(&mut row));
        } else {
            cell.push(c);
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell.trim().to_string());
        rows.push(row);
    }
    rows
}

/// Drop empty rows, and empty columns at the right.
fn trim(rows: &mut Vec<Vec<String>>) {
    rows.retain(|r| r.iter().any(|c| !c.trim().is_empty()));
    for r in rows.iter_mut() {
        while r.last().is_some_and(|c| c.trim().is_empty()) {
            r.pop();
        }
    }
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// Other words for column titles, as other programs and people write them.
const ALIASES: &[(&str, &[&str])] = &[
    ("ch", &["channel", "chan", "kanal", "input", "in", "no", "nr", "number", "#"]),
    ("name", &["title", "label", "kanalname", "channelname"]),
    ("title", &["song", "name", "titel"]),
    ("mic", &["micdi", "mikro", "mikrofon", "microphone", "di"]),
    ("micdi", &["mic", "mikro", "mikrofon", "microphone"]),
    ("source", &["instrument", "quelle"]),
    ("stand", &["stativ"]),
    ("short", &["kurz", "kürzel", "kuerzel", "abbr", "initials", "abbreviation"]),
    ("vocalrange", &["range", "voice", "voicetype", "stimmlage", "stimme", "stimmumfang", "tessitura", "fach"]),
    ("48v", &["phantom", "p48"]),
    ("notes", &["note", "comment", "comments", "bemerkung", "notizen", "remarks"]),
    ("length", &["duration", "dauer", "time"]),
    ("phone", &["tel", "telefon", "mobile", "handy"]),
    ("email", &["mail", "emailaddress"]),
    ("function", &["role", "job", "position", "funktion"]),
    ("colour", &["color", "farbe"]),
    ("count", &["qty", "quantity", "anzahl", "amount"]),
    ("what", &["item", "artikel"]),
    ("start", &["time", "begin", "uhrzeit"]),
];

/// Does a source title (normalised) fit a column title?
type TitleMatch = dyn Fn(&str, &str) -> bool;

/// The table column for each source column (by the titles in `head`).
pub fn auto_map(head: &[String], columns: &[Column]) -> Vec<Option<usize>> {
    let titles: Vec<String> = columns.iter().map(|c| norm(&c.title)).collect();
    // Worked out columns take nothing.
    let mut used: Vec<bool> = columns.iter().map(|c| c.kind == crate::grid::ColKind::Fixed).collect();
    let mut out = vec![None; head.len()];
    // Exact titles first, then the other words, then a title that starts the other.
    let passes: [&TitleMatch; 3] = [
        &|h, t| h == t,
        &|h, t| ALIASES.iter().any(|(k, words)| (t == *k || t.ends_with(k)) && words.contains(&h)),
        &|h, t| h.len() >= 3 && t.len() >= 3 && (t.starts_with(h) || h.starts_with(t)),
    ];
    for pass in passes {
        for (i, h) in head.iter().enumerate() {
            if out[i].is_some() {
                continue;
            }
            let h = norm(h);
            if h.is_empty() {
                continue;
            }
            if let Some(c) = (0..titles.len()).find(|c| !used[*c] && pass(&h, &titles[*c])) {
                out[i] = Some(c);
                used[c] = true;
            }
        }
    }
    out
}

/// Does the first row look like titles (it names at least one column and
/// most of its cells are not numbers)?
pub fn has_header(first: &[String], columns: &[Column]) -> bool {
    let named = auto_map(first, columns).iter().filter(|m| m.is_some()).count();
    let numbers = first.iter().filter(|c| crate::time::parse_number(c).is_some()).count();
    named > 0 && numbers * 2 <= first.len()
}

/// What an import did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    pub rows: usize,
    /// "Row 3, Mic: …" for every value that was not taken.
    pub problems: Vec<String>,
}

/// Add `rows` at the end of the table; `map[i]` is the table column of
/// source column `i`. A row of which no value was taken is not added.
pub fn import(grid: &dyn Grid, p: &mut Project, rows: &[Vec<String>], map: &[Option<usize>], first_row: usize) -> Report {
    let columns = grid.columns(p);
    let mut report = Report::default();
    for (n, src) in rows.iter().enumerate() {
        let cells: Vec<(usize, &String)> = src
            .iter()
            .enumerate()
            .filter_map(|(i, v)| map.get(i).copied().flatten().map(|c| (c, v)))
            .filter(|(_, v)| !v.trim().is_empty())
            .collect();
        if cells.is_empty() {
            continue;
        }
        let at = grid.len(p);
        let row = grid.insert(p, at);
        let mut taken = 0;
        for (c, v) in cells {
            match grid.set(p, row, c, v) {
                Ok(()) => taken += 1,
                Err(e) => {
                    let title = columns.get(c).map_or("", |x| x.title.as_str());
                    report.problems.push(format!("Row {}, {title}: {e}", n + first_row));
                }
            }
        }
        if taken == 0 {
            grid.remove(p, &[row]);
        } else {
            report.rows += 1;
        }
    }
    report
}

/// The whole table as CSV (comma, quoted where needed), with its titles.
pub fn to_csv(grid: &dyn Grid, p: &Project) -> String {
    let columns = grid.columns(p);
    let quote = |s: &str| {
        if s.contains([',', '"', '\n', '\r']) || s.starts_with(' ') || s.ends_with(' ') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };
    let mut out = columns.iter().map(|c| quote(&c.title)).collect::<Vec<_>>().join(",");
    out.push_str("\r\n");
    for r in 0..grid.len(p) {
        let line: Vec<String> = (0..columns.len()).map(|c| quote(&grid.text(p, r, c))).collect();
        out.push_str(&line.join(","));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sheet::Sheet;

    #[test]
    fn text_with_any_delimiter_and_quotes() {
        let rows = parse_text("Name;Notes\r\n\"Doe; Jane\";\"says \"\"hi\"\"\"\nMax;\n");
        assert_eq!(rows, vec![vec!["Name", "Notes"], vec!["Doe; Jane", "says \"hi\""], vec!["Max", ""]]);
        let rows = parse_text("a,b\tc\nd,e\tf");
        assert_eq!(rows[0], vec!["a,b", "c"], "tab wins a tie");
        let rows = parse_text("\"multi\nline\",x");
        assert_eq!(rows, vec![vec!["multi\nline", "x"]]);
    }

    #[test]
    fn decoding() {
        assert_eq!(decode(b"\xef\xbb\xbfK\xc3\xb6ln"), "Köln");
        assert_eq!(decode(b"K\xf6ln \x80 \x96"), "Köln € –");
        assert_eq!(decode(&[0xff, 0xfe, b'h', 0, b'i', 0]), "hi");
    }

    #[test]
    fn excel_dates() {
        assert_eq!(excel_date(46613.0), "2027-08-14");
        assert_eq!(excel_date(46613.5), "2027-08-14 12:00");
    }

    #[test]
    fn titles_are_matched() {
        let p = crate::demo::band();
        let cols = crate::patch_grid::PatchGrid { sheet: crate::patch_grid::PatchSheet::Inputs, problems: &[] }.columns(&p);
        let head: Vec<String> = ["Channel", "Name", "Mikrofon", "Phantom", "Whatever", "Comments"].map(String::from).to_vec();
        let map = auto_map(&head, &cols);
        let title = |m: Option<usize>| m.map(|c| cols[c].title.clone());
        assert_eq!(title(map[0]).as_deref(), Some("Ch"));
        assert_eq!(title(map[1]).as_deref(), Some("Name"));
        assert!(title(map[2]).is_some_and(|t| t.starts_with("Mic")), "{:?}", title(map[2]));
        assert_eq!(title(map[3]).as_deref(), Some("48V"));
        assert_eq!(map[4], None);
        assert_eq!(title(map[5]).as_deref(), Some("Insert / notes"));
        assert!(has_header(&head, &cols));
        assert!(!has_header(&["1".into(), "Kick".into(), "2".into()], &cols));
    }

    #[test]
    fn importing_into_the_cast_and_back_out() {
        let mut p = Project::default();
        let rows = parse_text("Name\tShort\tNotes\nAnna Berg\tAB\tLead\n\t\t\nBen Kurz\tBK\t\n");
        let map = auto_map(&rows[0], &Grid::columns(&Sheet::Cast, &p));
        let r = import(&Sheet::Cast, &mut p, &rows[1..], &map, 2);
        assert_eq!(r.rows, 2, "{:?}", r.problems);
        assert_eq!(p.cast[1].name, "Ben Kurz");
        assert_eq!(p.cast[0].notes, "Lead");
        let csv = to_csv(&Sheet::Cast, &p);
        assert!(csv.lines().nth(1).is_some_and(|l| l.starts_with("Anna Berg,AB")), "{csv}");
        // Back in: the same rows again.
        let back = parse_text(&csv);
        let map = auto_map(&back[0], &Grid::columns(&Sheet::Cast, &p));
        let r = import(&Sheet::Cast, &mut p, &back[1..], &map, 2);
        assert_eq!(r.rows, 2);
        assert_eq!(p.cast.len(), 4);
    }

    #[test]
    fn bad_values_are_reported() {
        let mut p = crate::demo::band();
        let n = p.patch.inputs.len();
        let g = crate::patch_grid::PatchGrid { sheet: crate::patch_grid::PatchSheet::Inputs, problems: &[] };
        let cols = g.columns(&p);
        let ch = cols.iter().position(|c| c.title == "Ch").unwrap();
        let name = cols.iter().position(|c| c.title == "Name").unwrap();
        let rows = vec![vec!["40".to_string(), "Guest vox".to_string()], vec!["x".to_string(), String::new()]];
        let r = import(&g, &mut p, &rows, &[Some(ch), Some(name)], 1);
        assert_eq!(r.rows, 1);
        assert_eq!(p.patch.inputs.len(), n + 1, "the row with nothing taken is not added");
        assert_eq!(r.problems.len(), 1);
        assert!(r.problems[0].starts_with("Row 2, Ch:"), "{:?}", r.problems);
    }
}
