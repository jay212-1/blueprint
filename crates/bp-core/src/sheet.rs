//! The master data as tables: every list (cast, roles, scenes, transmitters,
//! capsules, locations) seen as rows and columns of text. The table editor,
//! copy and paste (TSV, as Excel and Numbers use it) and later CSV import all
//! go through this, so the parsing rules live in one place.

use crate::model::{
    Capsule, CapsuleKind, CastKind, Casting, Id, Location, Person, Project, Role, Scene,
    Transmitter, TxStatus, COLORS,
};
use crate::grid::{col, ColKind, Column, Grid};
use crate::time::{format_duration, format_number, parse_duration, parse_number};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sheet {
    Cast,
    Roles,
    Scenes,
    Transmitters,
    Capsules,
    Locations,
}

/// Anything with an id, so rows can be handled the same way.
trait Row: Clone + Default {
    fn id(&self) -> Id;
    fn set_id(&mut self, id: Id);
}

macro_rules! rows {
    ($($t:ty),*) => {$(
        impl Row for $t {
            fn id(&self) -> Id { self.id }
            fn set_id(&mut self, id: Id) { self.id = id; }
        }
    )*};
}
rows!(Person, Role, Scene, Transmitter, Capsule, Location);

/// Run `$body` with `$v` bound to the row vector of `$sheet` (shared or mutable).
macro_rules! with_rows {
    ($p:expr, $sheet:expr, |$v:ident| $body:expr) => {
        match $sheet {
            Sheet::Cast => { let $v = &$p.cast; $body }
            Sheet::Roles => { let $v = &$p.roles; $body }
            Sheet::Scenes => { let $v = &$p.scenes; $body }
            Sheet::Transmitters => { let $v = &$p.transmitters; $body }
            Sheet::Capsules => { let $v = &$p.capsules; $body }
            Sheet::Locations => { let $v = &$p.locations; $body }
        }
    };
}
macro_rules! with_rows_mut {
    ($p:expr, $sheet:expr, |$v:ident| $body:expr) => {
        match $sheet {
            Sheet::Cast => { let $v = &mut $p.cast; $body }
            Sheet::Roles => { let $v = &mut $p.roles; $body }
            Sheet::Scenes => { let $v = &mut $p.scenes; $body }
            Sheet::Transmitters => { let $v = &mut $p.transmitters; $body }
            Sheet::Capsules => { let $v = &mut $p.capsules; $body }
            Sheet::Locations => { let $v = &mut $p.locations; $body }
        }
    };
}

fn insert_row<T: Row>(v: &mut Vec<T>, at: usize, id: Id) -> usize {
    let at = at.min(v.len());
    let mut t = T::default();
    t.set_id(id);
    v.insert(at, t);
    at
}

pub(crate) fn remove_rows<T>(v: &mut Vec<T>, rows: &[usize]) {
    let mut i = 0;
    v.retain(|_| {
        let keep = !rows.contains(&i);
        i += 1;
        keep
    });
}

/// Move `rows` (as a block, order kept) so that they stand before the row
/// that was at `to`. Returns their new indices.
pub(crate) fn move_rows<T>(v: &mut Vec<T>, rows: &[usize], to: usize) -> Vec<usize> {
    let mut sorted: Vec<usize> = rows.iter().copied().filter(|r| *r < v.len()).collect();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.is_empty() {
        return Vec::new();
    }
    let before = sorted.iter().filter(|r| **r < to).count();
    let mut moving = Vec::new();
    for r in sorted.iter().rev() {
        moving.push(v.remove(*r));
    }
    moving.reverse();
    let at = to.saturating_sub(before).min(v.len());
    let n = moving.len();
    for (k, t) in moving.into_iter().enumerate() {
        v.insert(at + k, t);
    }
    (at..at + n).collect()
}

fn color_text(c: &str) -> String {
    c.to_string()
}

pub(crate) fn parse_color(v: &str) -> Result<String, String> {
    let v = v.trim().to_ascii_lowercase();
    let v = if v == "none" { String::new() } else { v };
    if COLORS.contains(&v.as_str()) {
        Ok(v)
    } else {
        Err(format!(
            "unknown colour \"{v}\" (use {})",
            COLORS[1..].join(", ")
        ))
    }
}

fn opt_duration(v: Option<f64>) -> String {
    v.map(format_duration).unwrap_or_default()
}

fn parse_opt_duration(v: &str) -> Result<Option<f64>, String> {
    if v.trim().is_empty() {
        return Ok(None);
    }
    parse_duration(v)
        .map(Some)
        .ok_or_else(|| format!("\"{}\" is not a time (use m:ss)", v.trim()))
}

/// "Anna Lee (alternate), Ben Ott (cover)" for the castings after the first lead.
fn others_text(p: &Project, r: &Role) -> String {
    let lead = first_lead(r);
    r.casting
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != lead)
        .map(|(_, c)| {
            let name = p.person(c.person).map(|x| x.name.as_str()).unwrap_or("?");
            format!("{name} ({})", c.kind.label())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn first_lead(r: &Role) -> Option<usize> {
    r.casting.iter().position(|c| c.kind == CastKind::Lead)
}

/// Parse "Name (kind), Name" (kind defaults to alternate).
fn parse_others(p: &Project, v: &str) -> Result<Vec<Casting>, String> {
    let mut out = Vec::new();
    for part in v.split([',', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (name, kind) = match part.rfind('(') {
            Some(i) if part.ends_with(')') => {
                let kind = &part[i + 1..part.len() - 1];
                let k = CastKind::parse(kind)
                    .ok_or_else(|| format!("unknown casting \"{kind}\" (lead, alternate, cover, swing)"))?;
                (part[..i].trim(), k)
            }
            _ => (part, CastKind::Alternate),
        };
        let person = p
            .person_named(name)
            .ok_or_else(|| format!("no cast member named \"{name}\""))?;
        out.push(Casting {
            person: person.id,
            kind,
        });
    }
    Ok(out)
}

/// "Mara, Lena" for role ids.
pub fn roles_text(p: &Project, roles: &[Id]) -> String {
    roles
        .iter()
        .filter_map(|r| p.role(*r))
        .map(|r| r.name.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Role names separated by commas (or semicolons) to ids.
pub fn parse_roles(p: &Project, v: &str) -> Result<Vec<Id>, String> {
    let mut out = Vec::new();
    for part in v.split([',', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let r = p
            .role_named(part)
            .ok_or_else(|| format!("no role named \"{part}\""))?;
        if !out.contains(&r.id) {
            out.push(r.id);
        }
    }
    Ok(out)
}

/// "yes" / "x" / the column's word → true, "" / "no" → false.
pub fn parse_flag(v: &str) -> Result<bool, String> {
    match v.trim().to_ascii_lowercase().as_str() {
        "" | "no" | "n" | "false" | "0" | "-" => Ok(false),
        "yes" | "y" | "x" | "true" | "1" | "spare" => Ok(true),
        other => Err(format!("\"{other}\" is not yes or no")),
    }
}

impl Grid for Sheet {
    fn key(&self) -> String {
        format!("{self:?}")
    }
    fn columns(&self, _p: &Project) -> Vec<Column> {
        Sheet::columns(*self)
    }
    fn len(&self, p: &Project) -> usize {
        Sheet::len(*self, p)
    }
    fn text(&self, p: &Project, row: usize, col: usize) -> String {
        Sheet::text(*self, p, row, col)
    }
    fn set(&self, p: &mut Project, row: usize, col: usize, value: &str) -> Result<(), String> {
        Sheet::set(*self, p, row, col, value)
    }
    fn color(&self, p: &Project, row: usize) -> String {
        Sheet::color(*self, p, row)
    }
    fn editable_rows(&self) -> bool {
        true
    }
    fn insert(&self, p: &mut Project, at: usize) -> usize {
        Sheet::insert(*self, p, at)
    }
    fn remove(&self, p: &mut Project, rows: &[usize]) {
        Sheet::remove(*self, p, rows)
    }
    fn move_rows(&self, p: &mut Project, rows: &[usize], to: usize) -> Vec<usize> {
        Sheet::move_rows(*self, p, rows, to)
    }
}

impl Sheet {
    pub const ALL: [Sheet; 6] = [
        Sheet::Cast,
        Sheet::Roles,
        Sheet::Scenes,
        Sheet::Transmitters,
        Sheet::Capsules,
        Sheet::Locations,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Sheet::Cast => "Cast",
            Sheet::Roles => "Roles",
            Sheet::Scenes => "Scenes",
            Sheet::Transmitters => "Transmitters",
            Sheet::Capsules => "Capsules",
            Sheet::Locations => "Locations",
        }
    }

    /// Singular, for buttons ("Add person").
    pub fn noun(self) -> &'static str {
        match self {
            Sheet::Cast => "person",
            Sheet::Roles => "role",
            Sheet::Scenes => "scene",
            Sheet::Transmitters => "transmitter",
            Sheet::Capsules => "capsule",
            Sheet::Locations => "location",
        }
    }

    pub fn columns(self) -> Vec<Column> {
        use ColKind::*;
        match self {
            Sheet::Cast => vec![
                col("Name", Text, 180.0),
                col("Short", Text, 70.0),
                col("Vocal range", Text, 130.0),
                col("Contact", Text, 200.0),
                col("Notes", Text, 260.0),
                col("Colour", Color, 90.0),
            ],
            Sheet::Roles => vec![
                col("Role", Text, 170.0),
                col("Played by", Person, 170.0),
                col("Also (alternate, cover, swing)", Text, 280.0),
                col("Notes", Text, 200.0),
                col("Colour", Color, 90.0),
            ],
            Sheet::Scenes => vec![
                col("Act", Text, 50.0),
                col("No.", Text, 60.0),
                col("Title / song", Text, 240.0),
                col("Page", Text, 60.0),
                col("Start", Duration, 70.0),
                col("Length", Duration, 70.0),
                col("On stage (roles)", Text, 280.0),
                col("Notes", Text, 200.0),
            ],
            Sheet::Transmitters => vec![
                col("Ch", Text, 50.0),
                col("Name", Text, 120.0),
                col("Model", Text, 140.0),
                col("Band", Text, 70.0),
                col("MHz", Number, 90.0),
                col("Battery", Text, 90.0),
                col(
                    "Status",
                    Choice(TxStatus::ALL.iter().map(|s| s.label().to_string()).collect()),
                    80.0,
                ),
                col("Colour", Color, 90.0),
                col("Notes", Text, 200.0),
            ],
            Sheet::Capsules => vec![
                col(
                    "Type",
                    Choice(CapsuleKind::ALL.iter().map(|s| s.label().to_string()).collect()),
                    90.0,
                ),
                col("Model", Text, 160.0),
                col("Colour", Text, 90.0),
                col("Serial", Text, 120.0),
                col("Spare", Choice(vec![String::new(), "spare".into()]), 70.0),
                col("Notes", Text, 240.0),
            ],
            Sheet::Locations => vec![
                col("Name", Text, 200.0),
                col("Notes", Text, 320.0),
                col("Colour", Color, 90.0),
            ],
        }
    }

    pub fn len(self, p: &Project) -> usize {
        with_rows!(p, self, |v| v.len())
    }

    pub fn is_empty(self, p: &Project) -> bool {
        self.len(p) == 0
    }

    pub fn id(self, p: &Project, row: usize) -> Option<Id> {
        with_rows!(p, self, |v| v.get(row).map(Row::id))
    }

    pub fn row_of(self, p: &Project, id: Id) -> Option<usize> {
        with_rows!(p, self, |v| v.iter().position(|r| r.id() == id))
    }

    /// The row's colour name ("" = none).
    pub fn color(self, p: &Project, row: usize) -> String {
        match self {
            Sheet::Cast => p.cast.get(row).map(|r| r.color.clone()),
            Sheet::Roles => p.roles.get(row).map(|r| r.color.clone()),
            Sheet::Transmitters => p.transmitters.get(row).map(|r| r.color.clone()),
            Sheet::Locations => p.locations.get(row).map(|r| r.color.clone()),
            Sheet::Scenes | Sheet::Capsules => None,
        }
        .unwrap_or_default()
    }

    /// The text shown in a cell.
    pub fn text(self, p: &Project, row: usize, column: usize) -> String {
        let s = match self {
            Sheet::Cast => p.cast.get(row).map(|r| match column {
                0 => r.name.clone(),
                1 => r.short.clone(),
                2 => r.vocal_range.clone(),
                3 => r.contact.clone(),
                4 => r.notes.clone(),
                5 => color_text(&r.color),
                _ => String::new(),
            }),
            Sheet::Roles => p.roles.get(row).map(|r| match column {
                0 => r.name.clone(),
                1 => first_lead(r)
                    .and_then(|i| p.person(r.casting[i].person))
                    .map(|x| x.name.clone())
                    .unwrap_or_default(),
                2 => others_text(p, r),
                3 => r.notes.clone(),
                4 => color_text(&r.color),
                _ => String::new(),
            }),
            Sheet::Scenes => p.scenes.get(row).map(|r| match column {
                0 => r.act.clone(),
                1 => r.number.clone(),
                2 => r.title.clone(),
                3 => r.page.clone(),
                4 => opt_duration(r.start),
                5 => opt_duration(r.duration),
                6 => roles_text(p, &r.roles),
                7 => r.notes.clone(),
                _ => String::new(),
            }),
            Sheet::Transmitters => p.transmitters.get(row).map(|r| match column {
                0 => r.channel.clone(),
                1 => r.name.clone(),
                2 => r.model.clone(),
                3 => r.band.clone(),
                4 => r.frequency.map(format_number).unwrap_or_default(),
                5 => r.battery.clone(),
                6 => r.status.label().to_string(),
                7 => color_text(&r.color),
                8 => r.notes.clone(),
                _ => String::new(),
            }),
            Sheet::Capsules => p.capsules.get(row).map(|r| match column {
                0 => r.kind.label().to_string(),
                1 => r.model.clone(),
                2 => r.color.clone(),
                3 => r.serial.clone(),
                4 => if r.spare { "spare".into() } else { String::new() },
                5 => r.notes.clone(),
                _ => String::new(),
            }),
            Sheet::Locations => p.locations.get(row).map(|r| match column {
                0 => r.name.clone(),
                1 => r.notes.clone(),
                2 => color_text(&r.color),
                _ => String::new(),
            }),
        };
        s.unwrap_or_default()
    }

    /// Set a cell from text. On a parse error nothing changes.
    pub fn set(self, p: &mut Project, row: usize, column: usize, value: &str) -> Result<(), String> {
        if row >= self.len(p) {
            return Err("no such row".into());
        }
        let v = value.trim_end_matches(['\r', '\n']);
        match self {
            Sheet::Cast => {
                let color = if column == 5 { parse_color(v)? } else { String::new() };
                let r = &mut p.cast[row];
                match column {
                    0 => r.name = v.trim().to_string(),
                    1 => r.short = v.trim().to_string(),
                    2 => r.vocal_range = v.trim().to_string(),
                    3 => r.contact = v.to_string(),
                    4 => r.notes = v.to_string(),
                    5 => r.color = color,
                    _ => {}
                }
            }
            Sheet::Roles => match column {
                0 => p.roles[row].name = v.trim().to_string(),
                1 => {
                    let person = if v.trim().is_empty() {
                        None
                    } else {
                        Some(
                            p.person_named(v)
                                .ok_or_else(|| format!("no cast member named \"{}\"", v.trim()))?
                                .id,
                        )
                    };
                    let r = &mut p.roles[row];
                    let lead = first_lead(r);
                    match (lead, person) {
                        (Some(i), Some(id)) => r.casting[i].person = id,
                        (Some(i), None) => {
                            r.casting.remove(i);
                        }
                        (None, Some(id)) => r.casting.insert(
                            0,
                            Casting {
                                person: id,
                                kind: CastKind::Lead,
                            },
                        ),
                        (None, None) => {}
                    }
                }
                2 => {
                    let others = parse_others(p, v)?;
                    let r = &mut p.roles[row];
                    let lead = first_lead(r).map(|i| r.casting[i]);
                    r.casting = lead.into_iter().chain(others).collect();
                }
                3 => p.roles[row].notes = v.to_string(),
                4 => p.roles[row].color = parse_color(v)?,
                _ => {}
            },
            Sheet::Scenes => {
                let time = if column == 4 || column == 5 { parse_opt_duration(v)? } else { None };
                let roles = if column == 6 { parse_roles(p, v)? } else { Vec::new() };
                let r = &mut p.scenes[row];
                match column {
                    0 => r.act = v.trim().to_string(),
                    1 => r.number = v.trim().to_string(),
                    2 => r.title = v.to_string(),
                    3 => r.page = v.trim().to_string(),
                    4 => r.start = time,
                    5 => r.duration = time,
                    6 => r.roles = roles,
                    7 => r.notes = v.to_string(),
                    _ => {}
                }
            }
            Sheet::Transmitters => {
                let r = &mut p.transmitters[row];
                match column {
                    0 => r.channel = v.trim().to_string(),
                    1 => r.name = v.trim().to_string(),
                    2 => r.model = v.trim().to_string(),
                    3 => r.band = v.trim().to_string(),
                    4 => {
                        r.frequency = if v.trim().is_empty() {
                            None
                        } else {
                            Some(parse_number(v).ok_or_else(|| {
                                format!("\"{}\" is not a frequency in MHz", v.trim())
                            })?)
                        }
                    }
                    5 => r.battery = v.trim().to_string(),
                    6 => {
                        let s = v.trim().to_ascii_lowercase();
                        r.status = TxStatus::ALL
                            .into_iter()
                            .find(|x| x.label() == s)
                            .ok_or_else(|| format!("unknown status \"{s}\" (active, spare, defect)"))?;
                    }
                    7 => r.color = parse_color(v)?,
                    8 => r.notes = v.to_string(),
                    _ => {}
                }
            }
            Sheet::Capsules => {
                let r = &mut p.capsules[row];
                match column {
                    0 => {
                        let s = v.trim().to_ascii_lowercase();
                        r.kind = CapsuleKind::ALL
                            .into_iter()
                            .find(|x| x.label() == s)
                            .ok_or_else(|| {
                                format!("unknown type \"{s}\" (headset, lavalier, handheld)")
                            })?;
                    }
                    1 => r.model = v.trim().to_string(),
                    2 => r.color = v.trim().to_string(),
                    3 => r.serial = v.trim().to_string(),
                    4 => r.spare = parse_flag(v)?,
                    5 => r.notes = v.to_string(),
                    _ => {}
                }
            }
            Sheet::Locations => {
                let color = if column == 2 { parse_color(v)? } else { String::new() };
                let r = &mut p.locations[row];
                match column {
                    0 => r.name = v.trim().to_string(),
                    1 => r.notes = v.to_string(),
                    2 => r.color = color,
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Insert an empty row at `at`; returns its index.
    pub fn insert(self, p: &mut Project, at: usize) -> usize {
        let id = p.alloc_id();
        with_rows_mut!(p, self, |v| insert_row(v, at, id))
    }

    /// Remove rows (and references to them).
    pub fn remove(self, p: &mut Project, rows: &[usize]) {
        with_rows_mut!(p, self, |v| remove_rows(v, rows));
        p.prune();
    }

    /// Move rows before the row at `to`; returns their new indices.
    pub fn move_rows(self, p: &mut Project, rows: &[usize], to: usize) -> Vec<usize> {
        with_rows_mut!(p, self, |v| move_rows(v, rows, to))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{copy, parse_tsv, paste, to_tsv};

    fn cast(p: &mut Project, names: &[&str]) {
        for n in names {
            let r = Sheet::Cast.insert(p, usize::MAX);
            Sheet::Cast.set(p, r, 0, n).unwrap();
        }
    }

    #[test]
    fn tsv_round_trip_with_quotes_and_line_breaks() {
        let grid = vec![
            vec!["a".to_string(), "b\tc".to_string()],
            vec!["line\nbreak".to_string(), "say \"hi\"".to_string()],
        ];
        let t = to_tsv(&grid);
        assert_eq!(parse_tsv(&t), grid);
        assert_eq!(parse_tsv("x\ty\r\n1\t2\r\n").len(), 2);
        assert_eq!(parse_tsv(""), Vec::<Vec<String>>::new());
    }

    #[test]
    fn paste_adds_rows_and_reports_bad_cells() {
        let mut p = Project::default();
        let (rows, errors) = paste(
            &Sheet::Transmitters,
            &mut p,
            0,
            0,
            "1\tAnna\tADX1M\tG57\t606,125\tSB900\tactive\n2\tBen\t\t\tlots\t\tbroken\n",
        );
        assert_eq!(rows, vec![0, 1]);
        assert_eq!(p.transmitters.len(), 2);
        assert_eq!(p.transmitters[0].frequency, Some(606.125));
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert_eq!(p.transmitters[1].name, "Ben");
        assert_eq!(p.transmitters[1].status, TxStatus::Active);
    }

    #[test]
    fn roles_cast_by_name_with_kinds() {
        let mut p = Project::default();
        cast(&mut p, &["Anna Lee", "Ben Ott", "Cleo"]);
        p.cast[1].short = "BO".into();
        let r = Sheet::Roles.insert(&mut p, 0);
        Sheet::Roles.set(&mut p, r, 0, "Mara").unwrap();
        Sheet::Roles.set(&mut p, r, 1, "anna lee").unwrap();
        Sheet::Roles.set(&mut p, r, 2, "BO (cover), Cleo").unwrap();
        assert_eq!(Sheet::Roles.text(&p, r, 1), "Anna Lee");
        assert_eq!(Sheet::Roles.text(&p, r, 2), "Ben Ott (cover), Cleo (alternate)");
        assert!(Sheet::Roles.set(&mut p, r, 2, "Nobody").is_err());
        assert_eq!(p.roles[0].casting.len(), 3, "a failed edit changes nothing");
        // Deleting a person removes them from the casting.
        Sheet::Cast.remove(&mut p, &[1]);
        assert_eq!(Sheet::Roles.text(&p, r, 2), "Cleo (alternate)");
        // Clearing "Played by" keeps the others.
        Sheet::Roles.set(&mut p, r, 1, "").unwrap();
        assert_eq!(p.roles[0].casting.len(), 1);
    }

    #[test]
    fn move_rows_as_a_block() {
        let mut p = Project::default();
        cast(&mut p, &["a", "b", "c", "d", "e"]);
        let names = |p: &Project| p.cast.iter().map(|x| x.name.clone()).collect::<Vec<_>>().join("");
        assert_eq!(Sheet::Cast.move_rows(&mut p, &[3, 1], 0), vec![0, 1]);
        assert_eq!(names(&p), "bdace");
        assert_eq!(Sheet::Cast.move_rows(&mut p, &[0], 5), vec![4]);
        assert_eq!(names(&p), "daceb");
    }

    #[test]
    fn scene_times_and_colours() {
        let mut p = Project::default();
        let r = Sheet::Scenes.insert(&mut p, 0);
        Sheet::Scenes.set(&mut p, r, 4, "12:30").unwrap();
        assert_eq!(Sheet::Scenes.text(&p, r, 4), "12:30");
        assert!(Sheet::Scenes.set(&mut p, r, 5, "soon").is_err());
        let c = Sheet::Cast.insert(&mut p, 0);
        assert!(Sheet::Cast.set(&mut p, c, 5, "Pink").is_err());
        Sheet::Cast.set(&mut p, c, 5, "Blue").unwrap();
        Sheet::Cast.set(&mut p, c, 2, " Mezzo ").unwrap();
        assert_eq!(Sheet::Cast.text(&p, c, 2), "Mezzo");
        assert_eq!(Sheet::Cast.color(&p, c), "blue");
        assert_eq!(copy(&Sheet::Cast, &p, &[c]), "\t\tMezzo\t\t\tblue");
    }
}
