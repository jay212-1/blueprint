//! The tables of the other plans: one [`PlanGrid`] per plan and part (a stage
//! plot's items, the run of show, contacts, the packing list, intercom
//! channels and people, a free table's rows and columns).

use crate::grid::{col, ColKind, Column, Grid};
use crate::model::{Id, Project, COLORS};
use crate::plans::{
    free_value, run_times, Access, Contact, Cue, FreeCol, FreeKind, FreeRow, IcChannel, IcKind, IcUser, PackRow, Plan,
    PlanKind,
};
use crate::sheet::{move_rows, parse_color, parse_flag, remove_rows};
use crate::stage::{channels, parse_channels, StageItem, Symbol};
use crate::time::{format_clock, format_duration, format_number, parse_clock, parse_duration, parse_number};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    StageItems,
    Run,
    Contacts,
    Pack,
    IcChannels,
    IcPeople,
    Rows,
    Columns,
}

impl Part {
    /// The tables a plan kind has (the first is shown first).
    pub fn of(kind: &PlanKind) -> &'static [Part] {
        match kind {
            PlanKind::StagePlot(_) => &[Part::StageItems],
            PlanKind::RunOfShow(_) => &[Part::Run],
            PlanKind::Contacts(_) => &[Part::Contacts],
            PlanKind::Packlist(_) => &[Part::Pack],
            PlanKind::Intercom(_) => &[Part::IcPeople, Part::IcChannels],
            PlanKind::Table(_) => &[Part::Rows, Part::Columns],
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Part::StageItems => "Items",
            Part::Run => "Schedule",
            Part::Contacts => "Contacts",
            Part::Pack => "Items",
            Part::IcChannels => "Channels",
            Part::IcPeople => "People",
            Part::Rows => "Rows",
            Part::Columns => "Columns",
        }
    }

    pub fn noun(self) -> &'static str {
        match self {
            Part::StageItems => "item",
            Part::Run => "row",
            Part::Contacts => "contact",
            Part::Pack => "item",
            Part::IcChannels => "channel",
            Part::IcPeople => "person",
            Part::Rows => "row",
            Part::Columns => "column",
        }
    }
}

pub struct PlanGrid {
    pub plan: Id,
    pub part: Part,
}

fn kind(p: &Project, id: Id) -> Option<&PlanKind> {
    p.plan(id).map(|x| &x.kind)
}

fn kind_mut(p: &mut Project, id: Id) -> Option<&mut PlanKind> {
    p.plan_mut(id).map(|x| &mut x.kind)
}

fn opt_clock(v: Option<f64>) -> String {
    v.map(format_clock).unwrap_or_default()
}

fn opt_dur(v: Option<f64>) -> String {
    v.map(format_duration).unwrap_or_default()
}

fn parse_opt<T>(t: &str, f: impl Fn(&str) -> Option<T>, what: &str) -> Result<Option<T>, String> {
    if t.trim().is_empty() {
        Ok(None)
    } else {
        f(t.trim()).map(Some).ok_or(format!("\"{}\" is not {what}", t.trim()))
    }
}

fn choice<T: Copy>(all: &[T], label: fn(T) -> &'static str) -> ColKind {
    ColKind::Choice(all.iter().map(|k| label(*k).to_string()).collect())
}

/// "T" talk and listen, "L" listen only.
fn access_text(a: Option<Access>) -> String {
    match a {
        Some(Access::Talk) => "Talk".into(),
        Some(Access::Listen) => "Listen".into(),
        None => String::new(),
    }
}

fn parse_access(v: &str) -> Result<Option<Access>, String> {
    match v.trim().to_ascii_lowercase().as_str() {
        "" | "-" | "no" => Ok(None),
        "t" | "talk" | "x" | "yes" | "tl" | "talk/listen" => Ok(Some(Access::Talk)),
        "l" | "listen" | "lo" => Ok(Some(Access::Listen)),
        o => Err(format!("\"{o}\" is not talk or listen")),
    }
}

/// Rows of the part, generically.
macro_rules! rows {
    ($p:expr, $g:expr, |$v:ident| $body:expr, $none:expr) => {
        match ($g.part, kind_mut($p, $g.plan)) {
            (Part::StageItems, Some(PlanKind::StagePlot(s))) => { let $v = &mut s.items; $body }
            (Part::Run, Some(PlanKind::RunOfShow(r))) => { let $v = &mut r.rows; $body }
            (Part::Contacts, Some(PlanKind::Contacts(c))) => { let $v = &mut c.rows; $body }
            (Part::Pack, Some(PlanKind::Packlist(k))) => { let $v = &mut k.rows; $body }
            (Part::IcChannels, Some(PlanKind::Intercom(i))) => { let $v = &mut i.channels; $body }
            (Part::IcPeople, Some(PlanKind::Intercom(i))) => { let $v = &mut i.people; $body }
            (Part::Rows, Some(PlanKind::Table(t))) => { let $v = &mut t.rows; $body }
            (Part::Columns, Some(PlanKind::Table(t))) => { let $v = &mut t.columns; $body }
            _ => $none,
        }
    };
}

impl Grid for PlanGrid {
    fn key(&self) -> String {
        format!("plan:{}:{:?}", self.plan, self.part)
    }

    fn columns(&self, p: &Project) -> Vec<Column> {
        use ColKind::*;
        match (self.part, kind(p, self.plan)) {
            (Part::StageItems, _) => vec![
                col("Symbol", choice(&Symbol::ALL, Symbol::label), 120.0),
                col("Label", Text, 160.0),
                col("Inputs (ch)", Text, 90.0),
                col("X (m)", Number, 60.0),
                col("Y (m)", Number, 60.0),
                col("Turn", Number, 55.0),
                col("Width", Number, 60.0),
                col("Depth", Number, 60.0),
                col("Colour", Color, 80.0),
            ],
            (Part::Run, _) => vec![
                col("Start", Text, 60.0),
                col("Length", Duration, 65.0),
                col("End", Fixed, 55.0),
                col("What", Text, 220.0),
                col("Who", Text, 140.0),
                col("Where", Text, 120.0),
                col("Cue", Text, 90.0),
                col("Notes", Text, 220.0),
                col("Colour", Color, 80.0),
            ],
            (Part::Contacts, _) => vec![
                col("Name", Text, 160.0),
                col("Function", Text, 150.0),
                col("Group", Text, 90.0),
                col("Company", Text, 130.0),
                col("Phone", Text, 130.0),
                col("Email", Text, 190.0),
                col("Notes", Text, 200.0),
                col("Colour", Color, 80.0),
            ],
            (Part::Pack, _) => vec![
                col("Group", Text, 100.0),
                col("What", Text, 200.0),
                col("Count", Number, 55.0),
                col("Case", Text, 100.0),
                col("Packed", Choice(vec!["".into(), "yes".into()]), 60.0),
                col("Notes", Text, 240.0),
                col("From", Fixed, 75.0),
            ],
            (Part::IcChannels, _) => vec![
                col("Channel", Text, 140.0),
                col("Kind", choice(&IcKind::ALL, IcKind::label), 100.0),
                col("Port / number", Text, 110.0),
                col("Notes", Text, 240.0),
                col("Colour", Color, 80.0),
            ],
            (Part::IcPeople, Some(PlanKind::Intercom(i))) => {
                let mut v = vec![
                    col("Name", Text, 140.0),
                    col("Position", Text, 120.0),
                    col("Device", Text, 100.0),
                    col("Group", Text, 80.0),
                ];
                let access = Choice(vec!["".into(), "Talk".into(), "Listen".into()]);
                v.extend(i.channels.iter().map(|c| col(c.name.clone(), access.clone(), 70.0)));
                v.push(col("Notes", Text, 180.0));
                v
            }
            (Part::Columns, _) => vec![
                col("Column", Text, 160.0),
                col("Kind", choice(&FreeKind::ALL, FreeKind::label), 100.0),
                col("Choices (for Choice)", Text, 260.0),
                col("Width", Number, 60.0),
            ],
            (Part::Rows, Some(PlanKind::Table(t))) => t
                .columns
                .iter()
                .map(|c| {
                    let k = match c.kind {
                        FreeKind::Text | FreeKind::Time => Text,
                        FreeKind::Number => Number,
                        FreeKind::Duration => Duration,
                        FreeKind::Check => Choice(vec!["".into(), "yes".into()]),
                        FreeKind::Choice => Choice(std::iter::once(String::new()).chain(c.choices()).collect()),
                        FreeKind::Color => Color,
                    };
                    col(if c.name.is_empty() { "(untitled)".to_string() } else { c.name.clone() }, k, if c.width > 20.0 { c.width } else { 120.0 })
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn len(&self, p: &Project) -> usize {
        match (self.part, kind(p, self.plan)) {
            (Part::StageItems, Some(PlanKind::StagePlot(s))) => s.items.len(),
            (Part::Run, Some(PlanKind::RunOfShow(r))) => r.rows.len(),
            (Part::Contacts, Some(PlanKind::Contacts(c))) => c.rows.len(),
            (Part::Pack, Some(PlanKind::Packlist(k))) => k.rows.len(),
            (Part::IcChannels, Some(PlanKind::Intercom(i))) => i.channels.len(),
            (Part::IcPeople, Some(PlanKind::Intercom(i))) => i.people.len(),
            (Part::Rows, Some(PlanKind::Table(t))) => t.rows.len(),
            (Part::Columns, Some(PlanKind::Table(t))) => t.columns.len(),
            _ => 0,
        }
    }

    fn text(&self, p: &Project, row: usize, c: usize) -> String {
        match (self.part, kind(p, self.plan)) {
            (Part::StageItems, Some(PlanKind::StagePlot(s))) => {
                let Some(it) = s.items.get(row) else { return String::new() };
                match c {
                    0 => it.symbol.label().into(),
                    1 => it.label.clone(),
                    2 => channels(p, it),
                    3 => format_number(it.x),
                    4 => format_number(it.y),
                    5 => format_number(it.rot),
                    6 => format_number(it.w),
                    7 => format_number(it.d),
                    8 => it.color.clone(),
                    _ => String::new(),
                }
            }
            (Part::Run, Some(PlanKind::RunOfShow(r))) => {
                let Some(x) = r.rows.get(row) else { return String::new() };
                let times = run_times(r);
                match c {
                    0 => opt_clock(x.start.or(times.get(row).and_then(|t| t.0))),
                    1 => opt_dur(x.duration),
                    2 => opt_clock(times.get(row).and_then(|t| t.1).filter(|_| x.duration.is_some())),
                    3 => x.what.clone(),
                    4 => x.who.clone(),
                    5 => x.location.clone(),
                    6 => x.cue.clone(),
                    7 => x.notes.clone(),
                    8 => x.color.clone(),
                    _ => String::new(),
                }
            }
            (Part::Contacts, Some(PlanKind::Contacts(cs))) => {
                let Some(x) = cs.rows.get(row) else { return String::new() };
                [&x.name, &x.function, &x.group, &x.company, &x.phone, &x.email, &x.notes, &x.color]
                    .get(c)
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            }
            (Part::Pack, Some(PlanKind::Packlist(k))) => {
                let Some(x) = k.rows.get(row) else { return String::new() };
                match c {
                    0 => x.group.clone(),
                    1 => x.what.clone(),
                    2 => x.count.to_string(),
                    3 => x.case.clone(),
                    4 => if x.packed { "yes".into() } else { String::new() },
                    5 => x.notes.clone(),
                    6 => x.source.clone(),
                    _ => String::new(),
                }
            }
            (Part::IcChannels, Some(PlanKind::Intercom(i))) => {
                let Some(x) = i.channels.get(row) else { return String::new() };
                match c {
                    0 => x.name.clone(),
                    1 => x.kind.label().into(),
                    2 => x.number.clone(),
                    3 => x.notes.clone(),
                    4 => x.color.clone(),
                    _ => String::new(),
                }
            }
            (Part::IcPeople, Some(PlanKind::Intercom(i))) => {
                let Some(x) = i.people.get(row) else { return String::new() };
                let n = i.channels.len();
                match c {
                    0 => x.name.clone(),
                    1 => x.position.clone(),
                    2 => x.device.clone(),
                    3 => x.group.clone(),
                    c if c >= 4 && c < 4 + n => access_text(x.on(i.channels[c - 4].id)),
                    c if c == 4 + n => x.notes.clone(),
                    _ => String::new(),
                }
            }
            (Part::Rows, Some(PlanKind::Table(t))) => {
                let (Some(r), Some(col)) = (t.rows.get(row), t.columns.get(c)) else { return String::new() };
                r.get(col.id).to_string()
            }
            (Part::Columns, Some(PlanKind::Table(t))) => {
                let Some(x) = t.columns.get(row) else { return String::new() };
                match c {
                    0 => x.name.clone(),
                    1 => x.kind.label().into(),
                    2 => x.options.clone(),
                    3 => if x.width > 0.0 { format_number(x.width as f64) } else { String::new() },
                    _ => String::new(),
                }
            }
            _ => String::new(),
        }
    }

    fn set(&self, p: &mut Project, row: usize, c: usize, v: &str) -> Result<(), String> {
        let t = v.trim();
        // Inputs of a stage item are looked up before borrowing the plan.
        let inputs = if self.part == Part::StageItems && c == 2 { Some(parse_channels(p, t)?) } else { None };
        let num = |what: &str| parse_number(t).ok_or(format!("\"{t}\" is not {what}"));
        match (self.part, kind_mut(p, self.plan)) {
            (Part::StageItems, Some(PlanKind::StagePlot(s))) => {
                let it = s.items.get_mut(row).ok_or("no such item")?;
                match c {
                    0 => {
                        let sym = Symbol::parse(t).ok_or(format!("\"{t}\" is not a symbol"))?;
                        if sym != it.symbol {
                            it.symbol = sym;
                            (it.w, it.d) = sym.size();
                        }
                    }
                    1 => it.label = t.into(),
                    2 => it.inputs = inputs.unwrap_or_default(),
                    3 => it.x = num("a position")?,
                    4 => it.y = num("a position")?,
                    5 => it.rot = num("an angle")?.rem_euclid(360.0),
                    6 => it.w = num("a size")?.max(0.05),
                    7 => it.d = num("a size")?.max(0.05),
                    8 => it.color = parse_color(t)?,
                    _ => return Err("no such column".into()),
                }
            }
            (Part::Run, Some(PlanKind::RunOfShow(r))) => {
                let x = r.rows.get_mut(row).ok_or("no such row")?;
                match c {
                    0 => x.start = parse_opt(t, parse_clock, "a time of day (use 14:00)")?,
                    1 => x.duration = parse_opt(t, parse_duration, "a length (use h:mm:ss or m:ss)")?,
                    2 => return Err("the end is worked out from start and length".into()),
                    3 => x.what = t.into(),
                    4 => x.who = t.into(),
                    5 => x.location = t.into(),
                    6 => x.cue = t.into(),
                    7 => x.notes = v.into(),
                    8 => x.color = parse_color(t)?,
                    _ => return Err("no such column".into()),
                }
            }
            (Part::Contacts, Some(PlanKind::Contacts(cs))) => {
                let x = cs.rows.get_mut(row).ok_or("no such contact")?;
                match c {
                    0 => x.name = t.into(),
                    1 => x.function = t.into(),
                    2 => x.group = t.into(),
                    3 => x.company = t.into(),
                    4 => x.phone = t.into(),
                    5 => x.email = t.into(),
                    6 => x.notes = v.into(),
                    7 => x.color = parse_color(t)?,
                    _ => return Err("no such column".into()),
                }
            }
            (Part::Pack, Some(PlanKind::Packlist(k))) => {
                let x = k.rows.get_mut(row).ok_or("no such item")?;
                let counted = !x.source.is_empty();
                match c {
                    0..=2 if counted => return Err(format!("counted from the {}: change it there (or add your own row)", x.source.to_lowercase())),
                    0 => x.group = t.into(),
                    1 => x.what = t.into(),
                    2 => x.count = if t.is_empty() { 0 } else { t.parse().map_err(|_| format!("\"{t}\" is not a count"))? },
                    3 => x.case = t.into(),
                    4 => x.packed = parse_flag(t)?,
                    5 => x.notes = v.into(),
                    _ => return Err("this column is worked out".into()),
                }
            }
            (Part::IcChannels, Some(PlanKind::Intercom(i))) => {
                let x = i.channels.get_mut(row).ok_or("no such channel")?;
                match c {
                    0 => x.name = t.into(),
                    1 => x.kind = IcKind::parse(t).ok_or(format!("\"{t}\" is not a kind (partyline, wireless, radio, program)"))?,
                    2 => x.number = t.into(),
                    3 => x.notes = v.into(),
                    4 => x.color = parse_color(t)?,
                    _ => return Err("no such column".into()),
                }
            }
            (Part::IcPeople, Some(PlanKind::Intercom(i))) => {
                let n = i.channels.len();
                let ch: Vec<Id> = i.channels.iter().map(|x| x.id).collect();
                let x = i.people.get_mut(row).ok_or("no such person")?;
                match c {
                    0 => x.name = t.into(),
                    1 => x.position = t.into(),
                    2 => x.device = t.into(),
                    3 => x.group = t.into(),
                    c if c >= 4 && c < 4 + n => x.set(ch[c - 4], parse_access(t)?),
                    c if c == 4 + n => x.notes = v.into(),
                    _ => return Err("no such column".into()),
                }
            }
            (Part::Rows, Some(PlanKind::Table(tb))) => {
                let col = tb.columns.get(c).cloned().ok_or("no such column")?;
                let value = free_value(&col, v)?;
                tb.rows.get_mut(row).ok_or("no such row")?.put(col.id, value);
            }
            (Part::Columns, Some(PlanKind::Table(tb))) => {
                let x = tb.columns.get_mut(row).ok_or("no such column")?;
                match c {
                    0 => x.name = t.into(),
                    1 => x.kind = FreeKind::parse(t).ok_or(format!("\"{t}\" is not a kind of column"))?,
                    2 => x.options = t.into(),
                    3 => x.width = if t.is_empty() { 0.0 } else { num("a width")?.clamp(30.0, 800.0) as f32 },
                    _ => return Err("no such column".into()),
                }
                // Cells that no longer fit the kind are cleared.
                let col = x.clone();
                for r in &mut tb.rows {
                    let old = r.get(col.id).to_string();
                    if !old.is_empty() {
                        let new = free_value(&col, &old).unwrap_or_default();
                        r.put(col.id, new);
                    }
                }
            }
            _ => return Err("this plan is gone".into()),
        }
        Ok(())
    }

    fn color(&self, p: &Project, row: usize) -> String {
        let c = match (self.part, kind(p, self.plan)) {
            (Part::StageItems, Some(PlanKind::StagePlot(s))) => s.items.get(row).map(|x| x.color.clone()),
            (Part::Run, Some(PlanKind::RunOfShow(r))) => r.rows.get(row).map(|x| x.color.clone()),
            (Part::Contacts, Some(PlanKind::Contacts(c))) => c.rows.get(row).map(|x| x.color.clone()),
            (Part::Pack, Some(PlanKind::Packlist(k))) => k.rows.get(row).map(|x| if x.packed { "green".into() } else { String::new() }),
            (Part::IcChannels, Some(PlanKind::Intercom(i))) => i.channels.get(row).map(|x| x.color.clone()),
            (Part::Rows, Some(PlanKind::Table(t))) => t
                .columns
                .iter()
                .find(|c| c.kind == FreeKind::Color)
                .and_then(|c| t.rows.get(row).map(|r| r.get(c.id).to_string())),
            _ => None,
        };
        c.filter(|c| COLORS.contains(&c.as_str())).unwrap_or_default()
    }

    fn editable_rows(&self) -> bool {
        true
    }

    fn insert(&self, p: &mut Project, at: usize) -> usize {
        let id = p.alloc_id();
        // A new stage item lands in the middle of the stage.
        let centre = match kind(p, self.plan) {
            Some(PlanKind::StagePlot(s)) => (s.width / 2.0, s.depth / 2.0),
            _ => (0.0, 0.0),
        };
        match (self.part, kind_mut(p, self.plan)) {
            (Part::StageItems, Some(PlanKind::StagePlot(s))) => {
                let at = at.min(s.items.len());
                s.items.insert(at, StageItem::new(id, Symbol::MicStand, centre.0, centre.1));
                at
            }
            (Part::Run, Some(PlanKind::RunOfShow(r))) => {
                let at = at.min(r.rows.len());
                r.rows.insert(at, Cue { id, ..Cue::default() });
                at
            }
            (Part::Contacts, Some(PlanKind::Contacts(c))) => {
                let at = at.min(c.rows.len());
                c.rows.insert(at, Contact { id, ..Contact::default() });
                at
            }
            (Part::Pack, Some(PlanKind::Packlist(k))) => {
                let at = at.min(k.rows.len());
                k.rows.insert(at, PackRow { id, count: 1, ..PackRow::default() });
                at
            }
            (Part::IcChannels, Some(PlanKind::Intercom(i))) => {
                let at = at.min(i.channels.len());
                i.channels.insert(at, IcChannel { id, ..IcChannel::default() });
                at
            }
            (Part::IcPeople, Some(PlanKind::Intercom(i))) => {
                let at = at.min(i.people.len());
                i.people.insert(at, IcUser { id, ..IcUser::default() });
                at
            }
            (Part::Rows, Some(PlanKind::Table(t))) => {
                let at = at.min(t.rows.len());
                t.rows.insert(at, FreeRow { id, ..FreeRow::default() });
                at
            }
            (Part::Columns, Some(PlanKind::Table(t))) => {
                let at = at.min(t.columns.len());
                t.columns.insert(at, FreeCol { id, name: format!("Column {}", t.columns.len() + 1), width: 140.0, ..FreeCol::default() });
                at
            }
            _ => at,
        }
    }

    fn remove(&self, p: &mut Project, rows: &[usize]) {
        rows!(p, self, |v| remove_rows(v, rows), ());
        p.prune();
    }

    fn move_rows(&self, p: &mut Project, rows: &[usize], to: usize) -> Vec<usize> {
        rows!(p, self, |v| move_rows(v, rows, to), rows.to_vec())
    }
}

/// The plan with this id, if it still exists (for pages and prints).
pub fn plan_of(p: &Project, id: Id) -> Option<&Plan> {
    p.plan(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::paste;
    use crate::plans::PlanType;

    #[test]
    fn a_free_table_with_its_own_columns() {
        let mut p = Project::default();
        let id = p.add_plan(PlanType::Table);
        let cols = PlanGrid { plan: id, part: Part::Columns };
        let rows = PlanGrid { plan: id, part: Part::Rows };
        let (_, e) = paste(&cols, &mut p, 2, 0, "Due\tTime\nDone\tcheck\nState\tChoice\tOpen, Closed");
        assert!(e.is_empty(), "{e:?}");
        assert_eq!(rows.columns(&p).len(), 5);
        let (_, e) = paste(&rows, &mut p, 0, 0, "Stage\tnotes\t1830\tx\topen\nBar\t\t25:00\t\tmaybe");
        assert_eq!(e.len(), 2, "{e:?}");
        assert_eq!(rows.text(&p, 0, 2), "18:30");
        assert_eq!(rows.text(&p, 0, 3), "yes");
        assert_eq!(rows.text(&p, 0, 4), "Open");
        // Changing a column's kind clears what no longer fits.
        cols.set(&mut p, 2, 1, "Number").unwrap();
        assert_eq!(rows.text(&p, 0, 2), "");
        cols.remove(&mut p, &[0]);
        assert_eq!(rows.columns(&p).len(), 4);
        assert_eq!(rows.text(&p, 0, 0), "notes");
    }

    #[test]
    fn intercom_matrix() {
        let mut p = Project::default();
        let id = p.add_plan(PlanType::Intercom);
        let ch = PlanGrid { plan: id, part: Part::IcChannels };
        let people = PlanGrid { plan: id, part: Part::IcPeople };
        paste(&ch, &mut p, 0, 0, "A Stage\tpartyline\t1\nB Light\twireless\t2");
        let (_, e) = paste(&people, &mut p, 0, 0, "Stage manager\tSM desk\tPanel\tStage\tT\tT\nLX op\tFOH\tBeltpack\tLight\tL\tTalk");
        assert!(e.is_empty(), "{e:?}");
        assert_eq!(people.text(&p, 1, 4), "Listen");
        assert_eq!(people.columns(&p)[5].title, "B Light");
        // Deleting a channel removes it from everyone.
        ch.remove(&mut p, &[0]);
        assert_eq!(people.text(&p, 1, 4), "Talk");
    }

    #[test]
    fn run_of_show_and_stage_items() {
        let mut p = crate::demo::band();
        let run = p.add_plan(PlanType::RunOfShow);
        let g = PlanGrid { plan: run, part: Part::Run };
        paste(&g, &mut p, 0, 0, "14:00\t2:00:00\t\tGet-in\nx\t45:00\t\tSoundcheck");
        assert_eq!(g.len(&p), 2);
        g.set(&mut p, 1, 0, "").unwrap();
        assert_eq!(g.text(&p, 1, 0), "16:00", "follows the row above");
        assert_eq!(g.text(&p, 1, 2), "16:45");
        let sp = p.add_plan(PlanType::StagePlot);
        let s = PlanGrid { plan: sp, part: Part::StageItems };
        s.insert(&mut p, 0);
        s.set(&mut p, 0, 2, "22").unwrap();
        assert_eq!(s.text(&p, 0, 2), "22");
        assert_eq!(s.text(&p, 0, 3), "5");
        s.set(&mut p, 0, 0, "wedge").unwrap();
        assert_eq!(s.text(&p, 0, 6), "0.6");
        assert!(s.set(&mut p, 0, 2, "77").is_err());
        // Deleting the input clears the link.
        p.patch.inputs.retain(|i| i.no != 22);
        p.prune();
        assert_eq!(s.text(&p, 0, 2), "");
    }

    #[test]
    fn counted_pack_rows_are_kept_apart() {
        let mut p = crate::demo::band();
        let id = p.add_plan(PlanType::Packlist);
        crate::plans::refresh_packlist(&mut p, id);
        let g = PlanGrid { plan: id, part: Part::Pack };
        assert!(g.set(&mut p, 0, 2, "5").is_err());
        g.set(&mut p, 0, 4, "yes").unwrap();
        assert_eq!(g.color(&p, 0), "green");
    }
}
